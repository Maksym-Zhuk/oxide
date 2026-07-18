use std::path::Path;

use anyhow::{Context, Result};
use colored::Colorize;

use crate::{context::AppContext, templates::AnesisTemplate, templates::install::install_template};

use super::cache::get_cached_template;

pub async fn template_info(ctx: &AppContext, template_name: &str, json: bool) -> Result<()> {
  let cached = get_cached_template(ctx, template_name)?;
  let manifest = match cached.filter(|c| ctx.paths.templates.join(&c.path).exists()) {
    Some(cached) => read_manifest(&ctx.paths.templates.join(&cached.path))?,
    None => {
      install_template(ctx, template_name).await?;
      read_manifest(&ctx.paths.templates.join(template_name))?
    }
  };

  if json {
    println!("{}", serde_json::to_string_pretty(&manifest)?);
    return Ok(());
  }

  println!(
    "{} {}",
    manifest.metadata.display_name.bold(),
    format!("({})", manifest.name).dimmed()
  );
  println!(
    "{} {} anesis {}",
    format!("v{}", manifest.version).cyan(),
    "·".dimmed(),
    manifest.anesis_version
  );
  if !manifest.metadata.description.is_empty() {
    println!("{}", manifest.metadata.description);
  }
  if !manifest.repository.url.is_empty() {
    println!("{} {}", "repository:".dimmed(), manifest.repository.url);
  }
  Ok(())
}

fn read_manifest(dir: &Path) -> Result<AnesisTemplate> {
  let path = dir.join("anesis.template.json");
  let content = std::fs::read_to_string(&path)
    .with_context(|| format!("Failed to read template manifest at {}", path.display()))?;
  serde_json::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))
}
