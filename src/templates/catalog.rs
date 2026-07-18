use anyhow::Result;
use serde::Deserialize;

use crate::{
  context::AppContext,
  utils::{
    http::fetch_paginated,
    picker::{ItemKind, PickItem},
    ui::spinner,
  },
};

use super::cache::read_installed_templates;

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

pub async fn fetch_catalog(ctx: &AppContext) -> Result<Vec<CatalogTemplate>> {
  fetch_paginated(ctx, "/template/all", "template catalog").await
}

impl CatalogTemplate {
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
