use std::path::Path;

use anyhow::{Context, Result};
use inquire::Confirm;

use crate::{context::AppContext, utils::fs::copy_dir_respecting_gitignore};

use super::{cache::update_addons_cache, manifest::AddonManifest};

/// Validates a local directory as an Anesis addon and copies it into the cache so
/// `anesis use <id> <command>` can run it without publishing to the registry. The
/// validation mirrors the server's: the directory must contain a well-formed
/// `anesis.addon.json`. Returns the addon id it was cached under, or `None` if
/// the user declined to overwrite an existing cached addon.
pub fn link_addon(ctx: &AppContext, source: &Path, force: bool) -> Result<Option<String>> {
  let manifest_path = source.join("anesis.addon.json");
  let content = std::fs::read_to_string(&manifest_path).with_context(|| {
    format!(
      "No 'anesis.addon.json' found at {}",
      manifest_path.display()
    )
  })?;
  let manifest: AddonManifest =
    serde_json::from_str(&content).context("Invalid anesis.addon.json structure")?;

  let dest = ctx.paths.addons.join(&manifest.id);
  if dest.exists() {
    if !force
      && !Confirm::new(&format!(
        "An addon named '{}' is already cached and will be overwritten. Continue?",
        manifest.id
      ))
      .with_default(false)
      .prompt()?
    {
      return Ok(None);
    }
    std::fs::remove_dir_all(&dest).with_context(|| {
      format!(
        "Failed to clear existing cached addon at {}",
        dest.display()
      )
    })?;
  }
  copy_dir_respecting_gitignore(source, &dest)
    .with_context(|| format!("Failed to copy addon from {}", source.display()))?;

  update_addons_cache(&ctx.paths.addons, &manifest.id, &manifest, "local")?;
  Ok(Some(manifest.id))
}
