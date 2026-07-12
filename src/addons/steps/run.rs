use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};
use colored::Colorize;
use inquire::Confirm;

use crate::addons::manifest::RunStep;

use super::Rollback;

/// Runs an arbitrary shell command in the project root. Because this executes
/// unsandboxed code, the exact command is always shown and an explicit yes is
/// required before it runs (bypassed by `--yes`/non-interactive).
pub fn execute_run(
  step: &RunStep,
  project_root: &Path,
  ctx: &tera::Context,
  non_interactive: bool,
) -> Result<Vec<Rollback>> {
  let command = super::render_string(&step.command, ctx)?;

  if !step.description.is_empty() {
    println!("  {}", step.description.dimmed());
  }
  println!("  {} {}", "will run:".dimmed(), command.yellow());
  if !non_interactive
    && !Confirm::new("Run this command?")
      .with_default(false)
      .prompt()?
  {
    bail!("run step declined: '{command}'");
  }

  let status = Command::new("sh")
    .arg("-c")
    .arg(&command)
    .current_dir(project_root)
    .status()
    .with_context(|| format!("failed to launch shell for '{command}'"))?;
  if !status.success() {
    bail!("command '{command}' exited with {status}");
  }

  // Non-reversible: record it so `undo` warns instead of silently pretending the
  // command's side effects were rolled back.
  Ok(vec![Rollback::IrreversibleRun { command }])
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn runs_in_project_root_and_records_irreversible() {
    let dir = std::env::temp_dir().join(format!("anesis-run-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let step = RunStep {
      command: "echo hi > marker.txt".to_string(),
      description: String::new(),
    };
    let rb = execute_run(&step, &dir, &tera::Context::new(), true).unwrap();
    assert!(
      dir.join("marker.txt").exists(),
      "command ran in project root"
    );
    assert!(matches!(rb.as_slice(), [Rollback::IrreversibleRun { .. }]));
    std::fs::remove_dir_all(&dir).unwrap();
  }

  #[test]
  fn nonzero_exit_is_error() {
    let step = RunStep {
      command: "exit 3".to_string(),
      description: String::new(),
    };
    let out = execute_run(&step, &std::env::temp_dir(), &tera::Context::new(), true);
    assert!(out.is_err());
  }
}
