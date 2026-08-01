use std::{
  collections::HashMap,
  path::{Path, PathBuf},
  process::Command,
};

use anyhow::{Context, Result, bail};
use colored::Colorize;

use crate::{context::AppContext, utils::fs::copy_dir_respecting_gitignore};

use super::runner::run_addon_command;

pub async fn test_addon(
  ctx: &AppContext,
  addon_id: &str,
  command: &str,
  project: Option<String>,
) -> Result<()> {
  let fixture = resolve_fixture(ctx, addon_id, project)?;

  let baseline = tempfile::Builder::new()
    .prefix("anesis-test-base-")
    .tempdir()?;
  let work = tempfile::Builder::new()
    .prefix("anesis-test-work-")
    .tempdir()?;
  copy_dir_respecting_gitignore(&fixture, baseline.path())
    .with_context(|| format!("failed to copy fixture from {}", fixture.display()))?;
  copy_dir_respecting_gitignore(&fixture, work.path())
    .with_context(|| format!("failed to copy fixture from {}", fixture.display()))?;

  println!(
    "Testing {} {} on a copy of {} (original untouched)...\n",
    addon_id.cyan(),
    command.cyan(),
    fixture.display()
  );

  run_addon_command(
    ctx,
    addon_id,
    command,
    work.path(),
    &HashMap::new(),
    true,
    false,
  )
  .await?;

  println!("\n{}", "Diff (changes the addon made):".bold());
  show_diff(baseline.path(), work.path());
  Ok(())
}

#[doc(hidden)]
pub fn resolve_fixture_for_tests(
  ctx: &AppContext,
  addon_id: &str,
  project: Option<String>,
) -> Result<PathBuf> {
  resolve_fixture(ctx, addon_id, project)
}

fn resolve_fixture(ctx: &AppContext, addon_id: &str, project: Option<String>) -> Result<PathBuf> {
  if let Some(p) = project {
    let path = PathBuf::from(&p);
    if !path.is_dir() {
      bail!("--project '{p}' is not a directory");
    }
    return Ok(path);
  }
  let fixture = ctx.paths.addons.join(addon_id).join("test-fixture");
  if fixture.is_dir() {
    return Ok(fixture);
  }
  bail!(
    "No fixture project. Pass --project <path> or ship a 'test-fixture/' directory with the addon."
  )
}

fn show_diff(baseline: &Path, work: &Path) {
  let Ok(diff) = which::which("diff") else {
    eprintln!(
      "('diff' was not found on PATH; cannot show changes. \
       On Windows, install Git for Windows or run this inside WSL.)"
    );
    eprintln!("  baseline: {}", baseline.display());
    eprintln!("  after:    {}", work.display());
    return;
  };

  match Command::new(diff)
    .arg("-ruN")
    .arg("-x")
    .arg("anesis.lock")
    .arg(baseline)
    .arg(work)
    .output()
  {
    Ok(out) => {
      let text = String::from_utf8_lossy(&out.stdout);
      if text.trim().is_empty() {
        println!("(the addon made no changes)");
      } else {
        print!("{text}");
      }
    }
    Err(_) => eprintln!("(system 'diff' not available; cannot show changes)"),
  }
}
