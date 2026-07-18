use anyhow::Result;
use serde::Deserialize;

use crate::{
  addons::cache::read_installed_addons,
  context::AppContext,
  utils::{
    http::fetch_paginated,
    picker::{ItemKind, PickItem},
    ui::spinner,
  },
};

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

impl CatalogAddon {
  fn haystack(&self) -> String {
    format!(
      "{} {} {}",
      self.addon_id, self.name, self.config.description
    )
    .to_lowercase()
  }

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

pub async fn fetch_addon_catalog(ctx: &AppContext) -> Result<Vec<CatalogAddon>> {
  fetch_paginated(ctx, "/addon/all", "addon catalog").await
}

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
