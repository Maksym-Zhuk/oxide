use anyhow::{Context, Result};
use serde::Deserialize;

use super::manifest::{StackManifest, validate};
use crate::{
  auth::token::get_auth_user,
  context::AppContext,
  utils::picker::{ItemKind, PickItem},
};

/// The server wraps a stack's manifest inside a `config` field (alongside
/// metadata like stars/owner); the CLI only needs the manifest itself.
#[derive(Deserialize)]
struct StackResponse {
  config: StackManifest,
}

/// A trimmed registry catalog row for `stack list`.
#[derive(Deserialize)]
pub struct CatalogStack {
  pub stack_id: String,
  pub name: String,
  #[serde(default)]
  pub description: String,
  #[serde(default)]
  pub star_count: i64,
  pub config: StackManifest,
}

#[derive(Deserialize)]
struct Paginated {
  data: Vec<CatalogStack>,
  total_pages: i64,
}

/// Fetches a stack's manifest from the registry (`GET /stack/{id}`).
pub async fn fetch_stack_manifest(ctx: &AppContext, stack_id: &str) -> Result<StackManifest> {
  let token = get_auth_user(&ctx.paths.auth).ok().map(|u| u.token);
  let mut req = ctx.client.get(format!("{}/stack/{stack_id}", ctx.backend_url));
  if let Some(token) = token {
    req = req.bearer_auth(token);
  }
  let res: StackResponse = req
    .send()
    .await
    .with_context(|| format!("Failed to fetch stack '{stack_id}' from registry"))?
    .error_for_status()
    .with_context(|| format!("Stack '{stack_id}' not found in registry"))?
    .json()
    .await
    .with_context(|| format!("Failed to parse stack '{stack_id}'"))?;
  validate(&res.config)?;
  Ok(res.config)
}

/// Fetches the full stack catalog (`GET /stack/all`), following pagination.
pub async fn fetch_stack_catalog(ctx: &AppContext) -> Result<Vec<CatalogStack>> {
  let token = get_auth_user(&ctx.paths.auth).ok().map(|u| u.token);
  let mut all = Vec::new();
  let mut page = 1;
  loop {
    let mut req = ctx
      .client
      .get(format!("{}/stack/all", ctx.backend_url))
      .query(&[("page", page), ("page_size", 100)]);
    if let Some(token) = &token {
      req = req.bearer_auth(token);
    }
    let res: Paginated = req
      .send()
      .await
      .context("Failed to connect to server for the stack catalog")?
      .error_for_status()
      .context("Server returned an error for the stack catalog")?
      .json()
      .await
      .context("Failed to parse the stack catalog")?;
    let total_pages = res.total_pages;
    all.extend(res.data);
    if page >= total_pages {
      break;
    }
    page += 1;
  }
  Ok(all)
}

/// Builds picker rows for the registry stack catalog, so stacks show up in
/// `anesis search` alongside templates and addons.
pub async fn stack_pick_items(ctx: &AppContext) -> Result<Vec<PickItem>> {
  let stacks = fetch_stack_catalog(ctx).await?;
  Ok(
    stacks
      .into_iter()
      .map(|s| {
        let meta = if s.star_count > 0 {
          format!(" · {}★", s.star_count)
        } else {
          String::new()
        };
        PickItem {
          kind: ItemKind::Stack,
          haystack: format!("{} {} {}", s.stack_id, s.name, s.description).to_lowercase(),
          id: s.stack_id,
          name: s.name,
          meta,
          description: s.description,
        }
      })
      .collect(),
  )
}
