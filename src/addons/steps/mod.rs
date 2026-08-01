use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub mod append;
pub mod copy;
pub mod create;
pub mod delete;
pub mod inject;
pub mod move_step;
pub mod packages;
pub mod rename;
pub mod replace;
pub mod run;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Rollback {
  DeleteCreatedFile { path: PathBuf },
  RestoreFile { path: PathBuf, original: Vec<u8> },
  RenameFile { from: PathBuf, to: PathBuf },
  IrreversibleRun { command: String },
}

#[derive(Debug)]
pub struct StepFailure {
  pub error: anyhow::Error,
  pub rollbacks: Vec<Rollback>,
}

impl StepFailure {
  pub fn new(error: impl Into<anyhow::Error>, rollbacks: Vec<Rollback>) -> Self {
    Self {
      error: error.into(),
      rollbacks,
    }
  }

  pub fn without_rollbacks(error: impl Into<anyhow::Error>) -> Self {
    Self::new(error, Vec::new())
  }
}

impl std::fmt::Display for StepFailure {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    std::fmt::Display::fmt(&self.error, f)
  }
}

impl std::error::Error for StepFailure {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    self.error.source()
  }
}

impl From<anyhow::Error> for StepFailure {
  fn from(error: anyhow::Error) -> Self {
    Self::without_rollbacks(error)
  }
}

pub type StepResult = std::result::Result<Vec<Rollback>, StepFailure>;

pub fn render_string(s: &str, ctx: &tera::Context) -> Result<String> {
  tera::Tera::one_off(s, ctx, false).map_err(Into::into)
}

fn normalize_join(root: &Path, relative: &str) -> PathBuf {
  let joined = root.join(relative);
  let mut out = PathBuf::new();
  for component in joined.components() {
    match component {
      Component::ParentDir => {
        out.pop();
      }
      Component::CurDir => {}
      c => out.push(c),
    }
  }
  out
}

fn deepest_existing_ancestor(path: &Path) -> PathBuf {
  let mut current = path;
  loop {
    if current.symlink_metadata().is_ok() {
      return current.to_path_buf();
    }
    match current.parent() {
      Some(parent) => current = parent,
      None => return current.to_path_buf(),
    }
  }
}

pub(super) fn safe_join(root: &Path, relative: &str, label: &str) -> Result<PathBuf> {
  let canon_root = root
    .canonicalize()
    .with_context(|| format!("Cannot resolve project root '{}'", root.display()))?;

  let candidate = normalize_join(&canon_root, relative);

  if !candidate.starts_with(&canon_root) {
    return Err(anyhow::anyhow!(
      "Path traversal blocked: {} '{}' would escape the root directory",
      label,
      relative
    ));
  }

  let canon_existing = deepest_existing_ancestor(&candidate)
    .canonicalize()
    .with_context(|| format!("Cannot resolve {} '{}'", label, relative))?;
  if !canon_existing.starts_with(&canon_root) {
    return Err(anyhow::anyhow!(
      "Path traversal blocked: {} '{}' resolves outside the root directory via a symlink",
      label,
      relative
    ));
  }

  Ok(candidate)
}

pub(super) fn resolve_target(
  target: &crate::addons::manifest::Target,
  project_root: &Path,
  ctx: &tera::Context,
) -> Result<Vec<PathBuf>> {
  use crate::addons::manifest::Target;
  match target {
    Target::File { file } => {
      let rendered = render_string(file, ctx)?;
      let path = safe_join(project_root, &rendered, "target file")?;
      Ok(vec![path])
    }
    Target::Glob { glob } => {
      let glob = render_string(glob, ctx)?;
      safe_join(project_root, &glob, "glob pattern")?;
      let pattern = project_root.join(&glob).to_string_lossy().to_string();
      let canonical_root = project_root
        .canonicalize()
        .with_context(|| format!("Cannot resolve project root '{}'", project_root.display()))?;
      let paths = glob::glob(&pattern)?
        .filter_map(|e| e.ok())
        .filter(|p| {
          p.canonicalize()
            .map(|cp| cp.starts_with(&canonical_root))
            .unwrap_or(false)
        })
        .collect();
      Ok(paths)
    }
  }
}
