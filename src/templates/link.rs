use std::path::Path;

use anyhow::{Context, Result};
use inquire::Confirm;

use crate::{
  cache::update_templates_cache,
  context::AppContext,
  templates::AnesisTemplate,
  utils::{fs::copy_dir_respecting_gitignore, validate::validate_template_name},
};

/// Validates a local directory as an Anesis template and copies it into the cache
/// so `anesis new` can use it without publishing to the registry. The validation
/// mirrors the server's: the directory must contain a well-formed
/// `anesis.template.json`. Returns the template name it was cached under, or
/// `None` if the user declined to overwrite an existing cached template.
pub fn link_template(ctx: &AppContext, source: &Path, force: bool) -> Result<Option<String>> {
  let manifest_path = source.join("anesis.template.json");
  let content = std::fs::read_to_string(&manifest_path).with_context(|| {
    format!(
      "No 'anesis.template.json' found at {}",
      manifest_path.display()
    )
  })?;
  let manifest: AnesisTemplate =
    serde_json::from_str(&content).context("Invalid anesis.template.json structure")?;
  validate_template_name(&manifest.name)?;

  let dest = ctx.paths.templates.join(&manifest.name);
  if dest.exists() {
    if !force
      && !Confirm::new(&format!(
        "A template named '{}' is already cached and will be overwritten. Continue?",
        manifest.name
      ))
      .with_default(false)
      .prompt()?
    {
      return Ok(None);
    }
    std::fs::remove_dir_all(&dest).with_context(|| {
      format!(
        "Failed to clear existing cached template at {}",
        dest.display()
      )
    })?;
  }
  copy_dir_respecting_gitignore(source, &dest)
    .with_context(|| format!("Failed to copy template from {}", source.display()))?;

  update_templates_cache(&ctx.paths.templates, Path::new(&manifest.name), "local")?;
  Ok(Some(manifest.name))
}
