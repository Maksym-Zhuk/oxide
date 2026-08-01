use std::{
  collections::HashMap,
  fs,
  path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use super::steps::Rollback;

const LOCK_FILE_NAME: &str = "anesis.lock";

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LockFile {
  pub addons: Vec<LockEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LockEntry {
  pub id: String,
  pub version: String,
  pub variant: String,
  pub commands_executed: Vec<String>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub journal: Vec<Rollback>,
  #[serde(default, skip_serializing_if = "HashMap::is_empty")]
  pub inputs: HashMap<String, String>,
}

impl LockFile {
  pub fn load(project_root: &Path) -> Result<Self> {
    let path = project_root.join(LOCK_FILE_NAME);
    if !path.exists() {
      return Ok(Self::default());
    }
    let contents = fs::read_to_string(&path)?;
    let lock: Self = serde_json::from_str(&contents)?;
    lock.validate_journal_paths(project_root)?;
    Ok(lock)
  }

  fn validate_journal_paths(&self, project_root: &Path) -> Result<()> {
    let canon_root = project_root.canonicalize().with_context(|| {
      format!(
        "Cannot resolve project root '{}' while validating anesis.lock",
        project_root.display()
      )
    })?;

    let check = |path: &PathBuf| -> Result<()> {
      if !path.starts_with(&canon_root) {
        bail!(
          "anesis.lock references a path outside the project ('{}') -- refusing to load. \
           This file may have been tampered with; delete it and re-run the addon install if you trust this project.",
          path.display()
        );
      }
      Ok(())
    };

    for entry in &self.addons {
      for rollback in &entry.journal {
        match rollback {
          Rollback::DeleteCreatedFile { path } => check(path)?,
          Rollback::RestoreFile { path, .. } => check(path)?,
          Rollback::RenameFile { from, to } => {
            check(from)?;
            check(to)?;
          }
          Rollback::IrreversibleRun { .. } => {}
        }
      }
    }

    Ok(())
  }

  pub fn save(&self, project_root: &Path) -> Result<()> {
    let path = project_root.join(LOCK_FILE_NAME);
    let contents = serde_json::to_string_pretty(self)?;
    fs::write(path, contents)?;
    Ok(())
  }

  pub fn is_command_executed(&self, addon_id: &str, command: &str) -> bool {
    self
      .addons
      .iter()
      .find(|e| e.id == addon_id)
      .map(|e| e.commands_executed.iter().any(|c| c == command))
      .unwrap_or(false)
  }

  pub fn addon_version(&self, addon_id: &str) -> Option<&str> {
    self
      .addons
      .iter()
      .find(|e| e.id == addon_id)
      .map(|entry| entry.version.as_str())
  }

  pub fn mark_command_executed(&mut self, addon_id: &str, command: &str) {
    if let Some(entry) = self.addons.iter_mut().find(|e| e.id == addon_id)
      && !entry.commands_executed.iter().any(|c| c == command)
    {
      entry.commands_executed.push(command.to_string());
    }
  }

  pub fn remove_addon(&mut self, addon_id: &str) {
    self.addons.retain(|e| e.id != addon_id);
  }

  pub fn upsert_entry(&mut self, entry: LockEntry) {
    if let Some(existing) = self.addons.iter_mut().find(|e| e.id == entry.id) {
      *existing = entry;
    } else {
      self.addons.push(entry);
    }
  }
}
