use std::path::Path;

use anyhow::{Context, Result};
use inquire::Confirm;

use crate::addons::manifest::{CopyStep, IfExists};

use super::Rollback;

pub fn execute_copy(
  step: &CopyStep,
  addon_dir: &Path,
  project_root: &Path,
  ctx: &tera::Context,
  non_interactive: bool,
) -> Result<Vec<Rollback>> {
  let rendered_src = super::render_string(&step.src, ctx)?;
  let rendered_dest = super::render_string(&step.dest, ctx)?;
  let src = super::safe_join(addon_dir, &rendered_src, "addon source")?;
  let dest = super::safe_join(project_root, &rendered_dest, "copy destination")?;

  let mut rollbacks = Vec::new();

  if dest.exists() {
    match step.if_exists {
      IfExists::Skip => return Ok(rollbacks),
      IfExists::Ask => {
        if non_interactive {
          println!("  {rendered_dest} already exists — keeping it (pass no --yes to be asked)");
          return Ok(rollbacks);
        }
        let overwrite = Confirm::new(&format!("{} already exists. Overwrite?", rendered_dest))
          .with_default(false)
          .prompt()?;
        if !overwrite {
          return Ok(rollbacks);
        }
        rollbacks.push(Rollback::RestoreFile {
          path: dest.clone(),
          original: std::fs::read(&dest)?,
        });
      }
      IfExists::Overwrite => {
        rollbacks.push(Rollback::RestoreFile {
          path: dest.clone(),
          original: std::fs::read(&dest)?,
        });
      }
    }
  } else {
    rollbacks.push(Rollback::DeleteCreatedFile { path: dest.clone() });
  }

  if let Some(parent) = dest.parent() {
    std::fs::create_dir_all(parent)?;
  }

  if step.render {
    let bytes = std::fs::read(&src)
      .with_context(|| format!("Failed to read addon source {}", src.display()))?;
    if let Ok(text) = std::str::from_utf8(&bytes) {
      let rendered = super::render_string(text, ctx)?;
      std::fs::write(&dest, rendered)
        .with_context(|| format!("Failed to write {}", dest.display()))?;
      return Ok(rollbacks);
    }
  }

  let bytes = std::fs::read(&src)
    .with_context(|| format!("Failed to read addon source {}", src.display()))?;
  std::fs::write(&dest, bytes).with_context(|| format!("Failed to write {}", dest.display()))?;

  Ok(rollbacks)
}
