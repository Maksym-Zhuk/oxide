use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::addons::manifest::PackagesStep;

use super::Rollback;

/// The package managers we know how to drive. Detection is by lock-file (JS) then
/// by `Cargo.toml` (Rust), so an addon's `packages` step works in whatever project
/// it lands in without the author naming a manager.
enum PackageManager {
  Npm,
  Bun,
  Pnpm,
  Yarn,
  Cargo,
}

impl PackageManager {
  fn program(&self) -> &'static str {
    match self {
      PackageManager::Npm => "npm",
      PackageManager::Bun => "bun",
      PackageManager::Pnpm => "pnpm",
      PackageManager::Yarn => "yarn",
      PackageManager::Cargo => "cargo",
    }
  }

  /// Subcommand args that precede the package specs to add production deps.
  fn add_args(&self) -> &'static [&'static str] {
    match self {
      PackageManager::Npm => &["install"],
      _ => &["add"],
    }
  }

  /// Flag that turns an add into a dev-dependency add.
  fn dev_flag(&self) -> &'static str {
    match self {
      PackageManager::Npm | PackageManager::Pnpm => "--save-dev",
      _ => "--dev",
    }
  }

  /// Manifest + lock files to snapshot so the step is reversible. Only the ones
  /// that exist are actually snapshotted.
  fn snapshot_files(&self) -> &'static [&'static str] {
    match self {
      PackageManager::Cargo => &["Cargo.toml", "Cargo.lock"],
      PackageManager::Npm => &["package.json", "package-lock.json"],
      PackageManager::Bun => &["package.json", "bun.lock", "bun.lockb"],
      PackageManager::Pnpm => &["package.json", "pnpm-lock.yaml"],
      PackageManager::Yarn => &["package.json", "yarn.lock"],
    }
  }
}

fn detect_pm(root: &Path) -> Result<PackageManager> {
  // JS lock-files first (most specific), then a bare package.json → npm, then
  // Cargo.toml → cargo. A JS project never carries Cargo.toml, so this order is
  // unambiguous in practice.
  if root.join("bun.lock").exists() || root.join("bun.lockb").exists() {
    Ok(PackageManager::Bun)
  } else if root.join("pnpm-lock.yaml").exists() {
    Ok(PackageManager::Pnpm)
  } else if root.join("yarn.lock").exists() {
    Ok(PackageManager::Yarn)
  } else if root.join("package.json").exists() {
    Ok(PackageManager::Npm)
  } else if root.join("Cargo.toml").exists() {
    Ok(PackageManager::Cargo)
  } else {
    bail!("No package manager detected (no package.json or Cargo.toml in the project root)")
  }
}

pub fn execute_packages(step: &PackagesStep, project_root: &Path) -> Result<Vec<Rollback>> {
  if step.dependencies.is_empty() && step.dev_dependencies.is_empty() {
    return Ok(Vec::new());
  }
  let pm = detect_pm(project_root)?;

  // Snapshot the manifest + lock files up front: restoring them is the whole
  // reversal — it drops the added dependency entries. node_modules/target are left
  // alone (harmless; a later install reconciles them).
  let mut rollbacks = Vec::new();
  for name in pm.snapshot_files() {
    let path = project_root.join(name);
    if path.exists() {
      rollbacks.push(Rollback::RestoreFile {
        path: path.clone(),
        original: std::fs::read(&path)?,
      });
    }
  }

  let run = |extra: &[&str], specs: &[String]| -> Result<()> {
    let status = Command::new(pm.program())
      .args(pm.add_args())
      .args(extra)
      .args(specs)
      .current_dir(project_root)
      .status()
      .with_context(|| {
        format!(
          "failed to run '{}' — is it installed and on PATH?",
          pm.program()
        )
      })?;
    if !status.success() {
      bail!("'{}' exited with {}", pm.program(), status);
    }
    Ok(())
  };

  let result = (|| {
    if !step.dependencies.is_empty() {
      run(&[], &step.dependencies)?;
    }
    if !step.dev_dependencies.is_empty() {
      run(&[pm.dev_flag()], &step.dev_dependencies)?;
    }
    Ok(())
  })();

  if let Err(err) = result {
    // A failed install can leave the manifest half-edited; restore our snapshots
    // so the tree stays clean. (The runner discards rollbacks from a failing step,
    // so we must do this here rather than rely on it.)
    for rb in rollbacks.iter().rev() {
      if let Rollback::RestoreFile { path, original } = rb {
        let _ = std::fs::write(path, original);
      }
    }
    return Err(err);
  }

  Ok(rollbacks)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn detects_by_lockfile_then_manifest() {
    let dir = std::env::temp_dir().join(format!("anesis-pm-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    // Bare package.json → npm.
    std::fs::write(dir.join("package.json"), "{}").unwrap();
    assert!(matches!(detect_pm(&dir).unwrap(), PackageManager::Npm));

    // pnpm lock wins over the plain package.json.
    std::fs::write(dir.join("pnpm-lock.yaml"), "").unwrap();
    assert!(matches!(detect_pm(&dir).unwrap(), PackageManager::Pnpm));

    std::fs::remove_dir_all(&dir).unwrap();
    // No manifest at all → error.
    std::fs::create_dir_all(&dir).unwrap();
    assert!(detect_pm(&dir).is_err());
    std::fs::remove_dir_all(&dir).unwrap();
  }
}
