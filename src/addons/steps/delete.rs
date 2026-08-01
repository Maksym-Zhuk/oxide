use std::path::Path;

use crate::addons::manifest::DeleteStep;

use super::{Rollback, StepFailure, StepResult, resolve_target};

pub fn execute_delete(step: &DeleteStep, project_root: &Path, ctx: &tera::Context) -> StepResult {
  let paths = resolve_target(&step.target, project_root, ctx)?;
  let mut rollbacks = Vec::new();

  for path in paths {
    if path.is_dir() {
      eprintln!(
        "Warning: skipping '{}': deleting directories is not supported",
        path.display()
      );
      continue;
    }
    if !path.exists() {
      eprintln!(
        "Warning: delete target '{}' does not exist; skipping",
        path.display()
      );
      continue;
    }
    let original = match std::fs::read(&path) {
      Ok(o) => o,
      Err(e) => return Err(StepFailure::new(e, rollbacks)),
    };
    rollbacks.push(Rollback::RestoreFile {
      path: path.clone(),
      original,
    });
    if let Err(e) = std::fs::remove_file(&path) {
      return Err(StepFailure::new(e, rollbacks));
    }
  }

  Ok(rollbacks)
}
