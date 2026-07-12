use std::{
  collections::HashMap,
  path::{Path, PathBuf},
  process::Command,
};

use anyhow::{Context, Result, bail};
use colored::Colorize;

use crate::{context::AppContext, utils::fs::copy_dir_respecting_gitignore};

use super::runner::run_addon_command;

/// Runs an addon command against a throwaway copy of a fixture project and prints
/// the resulting diff, leaving the original untouched. Used to preview what an
/// addon does before applying it for real.
pub async fn test_addon(
  ctx: &AppContext,
  addon_id: &str,
  command: &str,
  project: Option<String>,
) -> Result<()> {
  let fixture = resolve_fixture(ctx, addon_id, project)?;

  // Two identical gitignore-respecting copies: run the addon on `work`, diff it
  // against the pristine `baseline`. Copying both the same way means anything the
  // addon didn't touch (node_modules, build output) is identical on both sides and
  // never shows up in the diff — no exclude guessing needed.
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

  // Non-interactive, default inputs; a temp copy so nothing real is modified.
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

fn resolve_fixture(ctx: &AppContext, addon_id: &str, project: Option<String>) -> Result<PathBuf> {
  if let Some(p) = project {
    let path = PathBuf::from(&p);
    if !path.is_dir() {
      bail!("--project '{p}' is not a directory");
    }
    return Ok(path);
  }
  // Fall back to a `test-fixture/` dir shipped alongside the cached addon.
  let fixture = ctx.paths.addons.join(addon_id).join("test-fixture");
  if fixture.is_dir() {
    return Ok(fixture);
  }
  bail!(
    "No fixture project. Pass --project <path> or ship a 'test-fixture/' directory with the addon."
  )
}

fn show_diff(baseline: &Path, work: &Path) {
  // Skip anesis.lock: its rollback journal embeds file contents as byte arrays,
  // which would bury the addon's actual code changes under thousands of noise lines.
  match Command::new("diff")
    .arg("-ruN")
    .arg("-x")
    .arg("anesis.lock")
    .arg(baseline)
    .arg(work)
    .output()
  {
    // diff exits 0 when identical, 1 when they differ; both are success for us.
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
