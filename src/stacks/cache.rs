use std::{
  fs,
  path::{Path, PathBuf},
};

use anyhow::{Result, anyhow};

use super::{
  manifest::{StackManifest, load_stack},
  registry::fetch_stack_manifest,
};
use crate::context::AppContext;

pub(crate) fn cached_path(ctx: &AppContext, stack_id: &str) -> PathBuf {
  ctx.paths.stacks.join(format!("{stack_id}.json"))
}

/// Fetches a stack manifest from the registry and caches it locally so
/// `new --stack <id>` works offline afterwards.
pub async fn install_stack(ctx: &AppContext, stack_id: &str) -> Result<StackManifest> {
  let manifest = fetch_stack_manifest(ctx, stack_id).await?;
  fs::create_dir_all(&ctx.paths.stacks)?;
  fs::write(cached_path(ctx, stack_id), serde_json::to_string_pretty(&manifest)?)?;
  Ok(manifest)
}

pub fn remove_cached_stack(ctx: &AppContext, stack_id: &str) -> Result<()> {
  let path = cached_path(ctx, stack_id);
  if !path.exists() {
    return Err(anyhow!("Stack '{stack_id}' is not installed locally"));
  }
  fs::remove_file(path)?;
  println!("✓ Removed stack '{stack_id}'");
  Ok(())
}

/// Reads the locally installed (cached) stacks.
pub fn read_installed_stacks(ctx: &AppContext) -> Result<Vec<StackManifest>> {
  let dir = &ctx.paths.stacks;
  if !dir.exists() {
    return Ok(Vec::new());
  }
  let mut out = Vec::new();
  for entry in fs::read_dir(dir)? {
    let path = entry?.path();
    if path.extension().and_then(|e| e.to_str()) == Some("json")
      && let Ok(m) = load_stack(&path)
    {
      out.push(m);
    }
  }
  Ok(out)
}

/// Resolves a `--stack` reference: an existing local file/dir wins, else a
/// previously-installed cached stack, else it's fetched from the registry.
pub async fn resolve_stack(ctx: &AppContext, reference: &str) -> Result<StackManifest> {
  let path = Path::new(reference);
  if path.exists() {
    return load_stack(path);
  }
  let cached = cached_path(ctx, reference);
  if cached.exists() {
    return load_stack(&cached);
  }
  install_stack(ctx, reference).await
}
