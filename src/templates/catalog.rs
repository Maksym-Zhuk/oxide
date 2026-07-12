use anyhow::{Context, Result};
use serde::Deserialize;

use crate::{
  auth::token::get_auth_user,
  cache::read_installed_templates,
  context::AppContext,
  utils::{
    picker::{ItemKind, PickItem},
    ui::spinner,
  },
};

/// A single catalog entry as rendered in the picker. `/template/all` groups
/// versions by name, so the display metadata lives under `latest.info`. Only
/// the fields we show are deserialized — the server's full row is much larger.
#[derive(Deserialize)]
pub struct CatalogTemplate {
  pub name: String,
  pub latest: CatalogLatest,
  #[serde(default)]
  pub star_count: i64,
}

#[derive(Deserialize)]
pub struct CatalogLatest {
  pub info: CatalogInfo,
}

impl CatalogTemplate {
  fn info(&self) -> &CatalogInfo {
    &self.latest.info
  }
}

#[derive(Deserialize)]
pub struct CatalogInfo {
  #[serde(rename = "displayName")]
  pub display_name: String,
  #[serde(default)]
  pub description: String,
  #[serde(default)]
  pub languages: Vec<String>,
}

#[derive(Deserialize)]
struct Paginated {
  data: Vec<CatalogTemplate>,
  total_pages: i64,
}

/// Fetches the full template catalog from the backend (the same `/template/all`
/// the website renders). Sends the bearer token when logged in so private
/// templates show up; works anonymously otherwise.
pub async fn fetch_catalog(ctx: &AppContext) -> Result<Vec<CatalogTemplate>> {
  let token = get_auth_user(&ctx.paths.auth).ok().map(|u| u.token);

  let mut all = Vec::new();
  let mut page = 1;
  loop {
    let mut req = ctx
      .client
      .get(format!("{}/template/all", ctx.backend_url))
      .query(&[("page", page), ("page_size", 100)]); // server clamps page_size to 100
    if let Some(token) = &token {
      req = req.bearer_auth(token);
    }

    let res: Paginated = req
      .send()
      .await
      .context("Failed to connect to server for the template catalog")?
      .error_for_status()
      .context("Server returned an error for the template catalog")?
      .json()
      .await
      .context("Failed to parse the template catalog")?;

    let total_pages = res.total_pages;
    all.extend(res.data);
    if page >= total_pages {
      break;
    }
    page += 1;
  }
  Ok(all)
}

impl CatalogTemplate {
  /// Text a `search` query is matched against (name, description, languages).
  fn haystack(&self) -> String {
    let info = self.info();
    format!(
      "{} {} {} {}",
      self.name,
      info.display_name,
      info.description,
      info.languages.join(" ")
    )
    .to_lowercase()
  }

  /// Builds the interactive-picker row for this template. `meta` is the
  /// ` · langs · N★` suffix drawn after the name.
  pub fn to_pick_item(&self) -> PickItem {
    let info = self.info();
    let mut meta = String::new();
    if !info.languages.is_empty() {
      meta.push_str(&format!(" · {}", info.languages.join(", ")));
    }
    if self.star_count > 0 {
      meta.push_str(&format!(" · {}★", self.star_count));
    }
    PickItem {
      kind: ItemKind::Template,
      id: self.name.clone(),
      name: self.name.clone(),
      meta,
      description: info.description.clone(),
      haystack: self.haystack(),
    }
  }
}

/// Builds the picker rows for the template picker. With `installed_only` it
/// reads the local cache; otherwise it fetches the full registry catalog.
pub async fn template_pick_items(ctx: &AppContext, installed_only: bool) -> Result<Vec<PickItem>> {
  if installed_only {
    Ok(
      read_installed_templates(&ctx.paths.templates)?
        .iter()
        .map(|t| t.to_pick_item())
        .collect(),
    )
  } else {
    let sp = spinner("Loading templates...");
    let templates = fetch_catalog(ctx).await?;
    sp.finish_and_clear();
    Ok(templates.iter().map(|t| t.to_pick_item()).collect())
  }
}
