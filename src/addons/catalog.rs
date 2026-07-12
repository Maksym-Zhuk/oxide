use anyhow::{Context, Result};
use serde::Deserialize;

use crate::{
  addons::cache::read_installed_addons,
  auth::token::get_auth_user,
  context::AppContext,
  utils::{
    picker::{ItemKind, PickItem},
    ui::spinner,
  },
};

/// A single addon registry entry, trimmed to the fields we display. The server's
/// full `Addon` row is much larger.
#[derive(Deserialize)]
pub struct CatalogAddon {
  pub addon_id: String,
  pub name: String,
  #[serde(default)]
  pub version: String,
  #[serde(default)]
  pub config: CatalogAddonConfig,
  #[serde(default)]
  pub star_count: i64,
}

#[derive(Deserialize, Default)]
pub struct CatalogAddonConfig {
  #[serde(default)]
  pub description: String,
}

#[derive(Deserialize)]
struct Paginated {
  data: Vec<CatalogAddon>,
  total_pages: i64,
}

impl CatalogAddon {
  fn haystack(&self) -> String {
    format!(
      "{} {} {}",
      self.addon_id, self.name, self.config.description
    )
    .to_lowercase()
  }

  /// Builds the interactive-picker row for this addon. `meta` is the
  /// ` · vVersion · N★` suffix drawn after the name.
  pub fn to_pick_item(&self) -> PickItem {
    let mut meta = String::new();
    if !self.version.is_empty() {
      meta.push_str(&format!(" · v{}", self.version));
    }
    if self.star_count > 0 {
      meta.push_str(&format!(" · {}★", self.star_count));
    }
    PickItem {
      kind: ItemKind::Addon,
      id: self.addon_id.clone(),
      name: self.name.clone(),
      meta,
      description: self.config.description.clone(),
      haystack: self.haystack(),
    }
  }
}

/// Fetches the full addon catalog from the backend (`/addon/all`). Sends the
/// bearer token when logged in so private addons show up; works anonymously.
pub async fn fetch_addon_catalog(ctx: &AppContext) -> Result<Vec<CatalogAddon>> {
  let token = get_auth_user(&ctx.paths.auth).ok().map(|u| u.token);

  let mut all = Vec::new();
  let mut page = 1;
  loop {
    let mut req = ctx
      .client
      .get(format!("{}/addon/all", ctx.backend_url))
      .query(&[("page", page), ("page_size", 100)]); // server clamps page_size to 100
    if let Some(token) = &token {
      req = req.bearer_auth(token);
    }

    let res: Paginated = req
      .send()
      .await
      .context("Failed to connect to server for the addon catalog")?
      .error_for_status()
      .context("Server returned an error for the addon catalog")?
      .json()
      .await
      .context("Failed to parse the addon catalog")?;

    let total_pages = res.total_pages;
    all.extend(res.data);
    if page >= total_pages {
      break;
    }
    page += 1;
  }
  Ok(all)
}

/// Builds the picker rows for the addon picker. With `installed_only` it reads
/// the local cache; otherwise it fetches the full registry catalog.
pub async fn addon_pick_items(ctx: &AppContext, installed_only: bool) -> Result<Vec<PickItem>> {
  if installed_only {
    Ok(
      read_installed_addons(&ctx.paths.addons)?
        .iter()
        .map(|a| a.to_pick_item())
        .collect(),
    )
  } else {
    let sp = spinner("Loading addons...");
    let addons = fetch_addon_catalog(ctx).await?;
    sp.finish_and_clear();
    Ok(addons.iter().map(|a| a.to_pick_item()).collect())
  }
}
