use std::path::Path;

use anyhow::{Result, anyhow};

use crate::addons::manifest::AppendStep;

use super::{Rollback, render_string, resolve_target};

pub fn execute_append(
  step: &AppendStep,
  project_root: &Path,
  ctx: &tera::Context,
) -> Result<Vec<Rollback>> {
  let paths = resolve_target(&step.target, project_root)?;
  let rendered = render_string(&step.content, ctx)?;

  let mut rollbacks = Vec::new();

  for path in paths {
    let original = std::fs::read(&path)?;
    let text = std::str::from_utf8(&original).map_err(|_| {
      anyhow!(
        "{} is not valid UTF-8 (binary file); refusing to append",
        path.display()
      )
    })?;
    let mut new_content = text.to_string();

    if !new_content.is_empty() && !new_content.ends_with('\n') {
      new_content.push('\n');
    }
    new_content.push_str(&rendered);

    rollbacks.push(Rollback::RestoreFile {
      path: path.clone(),
      original,
    });
    std::fs::write(&path, new_content)?;
  }

  Ok(rollbacks)
}
