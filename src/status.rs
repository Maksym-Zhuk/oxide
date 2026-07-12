use std::path::Path;

use anyhow::{Context, Result};
use colored::Colorize;

use crate::{addons::lock::LockFile, manifest::AnesisManifest};

/// Reads `anesis.json` (required — marks the project root) and `anesis.lock`
/// (optional — created once addons are applied) from `project_root`.
fn load(project_root: &Path) -> Result<(AnesisManifest, LockFile)> {
  let manifest_path = project_root.join("anesis.json");
  let contents = std::fs::read_to_string(&manifest_path).with_context(|| {
    "not an Anesis project (no anesis.json here). Run `anesis new` to create one, or cd into a project root."
  })?;
  let manifest: AnesisManifest = serde_json::from_str(&contents)?;
  let lock = LockFile::load(project_root)?;
  Ok((manifest, lock))
}

pub fn print_status(project_root: &Path) -> Result<()> {
  let (manifest, lock) = load(project_root)?;

  println!("{} {}", "template:".dimmed(), manifest.template_name.cyan());
  println!("{} {}", "sha:".dimmed(), manifest.template_sha);

  if lock.addons.is_empty() {
    println!("{} (none)", "addons:".dimmed());
    return Ok(());
  }

  println!("{}", "addons:".dimmed());
  for entry in &lock.addons {
    let version = if entry.version.is_empty() {
      String::new()
    } else {
      format!(" v{}", entry.version)
    };
    println!(
      "  {}{} [{}]",
      entry.id.cyan(),
      version.dimmed(),
      entry.variant.dimmed()
    );
    if !entry.commands_executed.is_empty() {
      println!(
        "    {} {}",
        "ran:".dimmed(),
        entry.commands_executed.join(", ")
      );
    }
  }
  Ok(())
}

pub fn status_json(project_root: &Path) -> Result<serde_json::Value> {
  let (manifest, lock) = load(project_root)?;
  Ok(serde_json::json!({
    "template": {
      "name": manifest.template_name,
      "sha": manifest.template_sha,
    },
    "addons": lock.addons.iter().map(|e| serde_json::json!({
      "id": e.id,
      "version": e.version,
      "variant": e.variant,
      "commands_executed": e.commands_executed,
    })).collect::<Vec<_>>(),
  }))
}
