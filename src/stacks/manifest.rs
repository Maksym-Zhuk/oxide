use std::{collections::HashMap, fs, path::Path};

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

/// A stack = a template plus an ordered list of addons (with pinned inputs).
/// Parsed from an `anesis.stack.json` file. See `registry/stacks/` for examples.
#[derive(Debug, Serialize, Deserialize)]
pub struct StackManifest {
  pub schema_version: String,
  pub id: String,
  pub name: String,
  #[serde(default)]
  pub description: String,
  pub template: String,
  #[serde(default)]
  pub addons: Vec<StackAddon>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StackAddon {
  pub id: String,
  #[serde(default = "default_command")]
  pub command: String,
  /// Inputs passed to the addon as presets, so the user isn't re-prompted for
  /// values the stack already fixed.
  #[serde(default)]
  pub inputs: HashMap<String, String>,
}

fn default_command() -> String {
  "install".to_string()
}

/// Reads and validates a stack manifest from a file path, or from a directory
/// containing an `anesis.stack.json`.
pub fn load_stack(path: &Path) -> Result<StackManifest> {
  let file = if path.is_dir() {
    path.join("anesis.stack.json")
  } else {
    path.to_path_buf()
  };
  let raw = fs::read_to_string(&file)
    .with_context(|| format!("could not read stack manifest at {}", file.display()))?;
  let stack: StackManifest =
    serde_json::from_str(&raw).with_context(|| format!("invalid stack manifest {}", file.display()))?;
  validate(&stack)?;
  Ok(stack)
}

/// Local, offline validation: shape only. Whether the referenced template/addons
/// actually exist is checked by the registry (server) on publish, and surfaced
/// with a clear error by the runner when applied.
pub fn validate(stack: &StackManifest) -> Result<()> {
  if stack.template.trim().is_empty() {
    return Err(anyhow!("stack '{}' declares no template", stack.id));
  }
  for (i, addon) in stack.addons.iter().enumerate() {
    if addon.id.trim().is_empty() {
      return Err(anyhow!("stack '{}': addon #{} has an empty id", stack.id, i + 1));
    }
  }
  Ok(())
}
