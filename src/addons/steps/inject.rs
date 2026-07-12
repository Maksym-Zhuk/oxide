use std::path::Path;

use anyhow::{Result, anyhow};
use inquire::Select;

use crate::addons::manifest::{IfNotFound, InjectStep};

use super::{Rollback, render_string, resolve_target};

pub fn execute_inject(
  step: &InjectStep,
  project_root: &Path,
  ctx: &tera::Context,
) -> Result<Vec<Rollback>> {
  if step.after.is_some() && step.before.is_some() {
    return Err(anyhow!(
      "inject step cannot set both 'after' and 'before'; choose one marker"
    ));
  }

  let paths = resolve_target(&step.target, project_root)?;
  let rendered: Vec<String> = render_string(&step.content, ctx)?
    .lines()
    .map(str::to_string)
    .collect();

  let mut rollbacks = Vec::new();

  for path in paths {
    let original = std::fs::read(&path)?;
    let text = std::str::from_utf8(&original).map_err(|_| {
      anyhow!(
        "{} is not valid UTF-8 (binary file); refusing to inject",
        path.display()
      )
    })?;
    let mut file_lines: Vec<String> = text.lines().map(str::to_string).collect();

    let marker = step.after.as_deref().or(step.before.as_deref());

    if let Some(marker) = marker {
      match file_lines.iter().position(|l| l.contains(marker)) {
        Some(idx) => {
          let insert_idx = if step.after.is_some() { idx + 1 } else { idx };
          for (i, line) in rendered.iter().enumerate() {
            file_lines.insert(insert_idx + i, line.clone());
          }
        }
        None => match step.if_not_found {
          IfNotFound::Skip => continue,
          IfNotFound::Error => {
            return Err(anyhow!(
              "Marker {:?} not found in {}",
              marker,
              path.display()
            ));
          }
          IfNotFound::WarnAndAsk => {
            eprintln!(
              "Warning: marker {:?} not found in {}",
              marker,
              path.display()
            );
            let choice =
              Select::new("How would you like to proceed?", vec!["Continue", "Abort"]).prompt()?;
            if choice == "Abort" {
              return Err(anyhow!("Aborted by user"));
            }
            continue;
          }
        },
      }
    } else {
      let mut new_lines = rendered.clone();
      new_lines.extend(file_lines);
      file_lines = new_lines;
    }

    let had_trailing_newline = original.last().copied() == Some(b'\n');
    rollbacks.push(Rollback::RestoreFile {
      path: path.clone(),
      original,
    });
    let mut new_content = file_lines.join("\n");
    if had_trailing_newline {
      new_content.push('\n');
    }
    std::fs::write(&path, new_content)?;
  }

  Ok(rollbacks)
}
