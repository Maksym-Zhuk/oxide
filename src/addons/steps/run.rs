use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};
use colored::Colorize;
use inquire::Confirm;

use crate::addons::manifest::RunStep;

use super::Rollback;

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

  Ok(vec![Rollback::IrreversibleRun { command }])
}
