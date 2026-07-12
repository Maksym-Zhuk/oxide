use std::path::Path;

use anyhow::{Result, anyhow};
use inquire::Select;

use crate::addons::manifest::{IfNotFound, ReplaceStep};

use super::{Rollback, render_string, resolve_target};

pub fn execute_replace(
  step: &ReplaceStep,
  project_root: &Path,
  ctx: &tera::Context,
) -> Result<Vec<Rollback>> {
  let paths = resolve_target(&step.target, project_root)?;
  let rendered_replace = render_string(&step.replace, ctx)?;

  let mut rollbacks = Vec::new();

  for path in paths {
    let original = std::fs::read(&path)?;
    let content = std::str::from_utf8(&original)
      .map_err(|_| {
        anyhow!(
          "{} is not valid UTF-8 (binary file); refusing to replace",
          path.display()
        )
      })?
      .to_string();

    if !content.contains(&step.find) {
      match step.if_not_found {
        IfNotFound::Skip => continue,
        IfNotFound::Error => {
          return Err(anyhow!(
            "Pattern {:?} not found in {}",
            step.find,
            path.display()
          ));
        }
        IfNotFound::WarnAndAsk => {
          eprintln!("Warning: {:?} not found in {}", step.find, path.display());
          let choice =
            Select::new("How would you like to proceed?", vec!["Continue", "Abort"]).prompt()?;
          if choice == "Abort" {
            return Err(anyhow!("Aborted by user"));
          }
          continue;
        }
      }
    }

    let new_content = content.replace(&step.find, &rendered_replace);
    rollbacks.push(Rollback::RestoreFile {
      path: path.clone(),
      original,
    });
    std::fs::write(&path, new_content)?;
  }

  Ok(rollbacks)
}
