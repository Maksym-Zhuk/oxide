use anyhow::{Context, Result};
use serde::Deserialize;

use super::manifest::{StackManifest, validate};
use crate::{
  auth::token::get_auth_user,
  context::AppContext,
  utils::{
    http::fetch_paginated,
    picker::{ItemKind, PickItem},
  },
};

#[derive(Deserialize)]
struct StackResponse {
  config: StackManifest,
}

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

pub async fn fetch_stack_manifest(ctx: &AppContext, stack_id: &str) -> Result<StackManifest> {
  let token = get_auth_user(&ctx.paths.auth).ok().map(|u| u.token);
  let mut req = ctx
    .client
    .get(format!("{}/stack/{stack_id}", ctx.backend_url));
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

pub async fn fetch_stack_catalog(ctx: &AppContext) -> Result<Vec<CatalogStack>> {
  fetch_paginated(ctx, "/stack/all", "stack catalog").await
}

impl CatalogStack {
  pub fn to_pick_item(&self) -> PickItem {
    let meta = if self.star_count > 0 {
      format!(" · {}★", self.star_count)
    } else {
      String::new()
    };
    PickItem {
      kind: ItemKind::Stack,
      haystack: format!("{} {} {}", self.stack_id, self.name, self.description).to_lowercase(),
      id: self.stack_id.clone(),
      name: self.name.clone(),
      meta,
      description: self.description.clone(),
    }
  }
}

pub async fn stack_pick_items(ctx: &AppContext) -> Result<Vec<PickItem>> {
  let stacks = fetch_stack_catalog(ctx).await?;
  Ok(stacks.iter().map(CatalogStack::to_pick_item).collect())
}
