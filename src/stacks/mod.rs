use std::{collections::HashMap, fs, path::Path};

use anyhow::{Context, Result, anyhow};
use colored::Colorize;
use serde::{Deserialize, Serialize};

use crate::{
  auth::token::get_auth_user,
  context::AppContext,
  utils::{
    picker::{ItemKind, PickItem},
    ui::spinner,
  },
};

/// A stack = a template plus an ordered list of addons (with pinned inputs).
/// Parsed from an `anesis.stack.json` file. See `registry/stacks/` for examples.
#[derive(Debug, Serialize, Deserialize)]
pub struct StackManifest {
  pub schema_version: String,
  pub id: String,
  pub name: String,
  #[serde(default)]
  pub description: String,
  pub template: String,
  #[serde(default)]
  pub addons: Vec<StackAddon>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StackAddon {
  pub id: String,
  #[serde(default = "default_command")]
  pub command: String,
  /// Inputs passed to the addon as presets, so the user isn't re-prompted for
  /// values the stack already fixed.
  #[serde(default)]
  pub inputs: HashMap<String, String>,
}

fn default_command() -> String {
  "install".to_string()
}

/// Reads and validates a stack manifest from a file path, or from a directory
/// containing an `anesis.stack.json`.
pub fn load_stack(path: &Path) -> Result<StackManifest> {
  let file = if path.is_dir() {
    path.join("anesis.stack.json")
  } else {
    path.to_path_buf()
  };
  let raw = fs::read_to_string(&file)
    .with_context(|| format!("could not read stack manifest at {}", file.display()))?;
  let stack: StackManifest =
    serde_json::from_str(&raw).with_context(|| format!("invalid stack manifest {}", file.display()))?;
  validate(&stack)?;
  Ok(stack)
}

/// Local, offline validation: shape only. Whether the referenced template/addons
/// actually exist is checked by the registry (server) on publish, and surfaced
/// with a clear error by the runner when applied.
fn validate(stack: &StackManifest) -> Result<()> {
  if stack.template.trim().is_empty() {
    return Err(anyhow!("stack '{}' declares no template", stack.id));
  }
  for (i, addon) in stack.addons.iter().enumerate() {
    if addon.id.trim().is_empty() {
      return Err(anyhow!("stack '{}': addon #{} has an empty id", stack.id, i + 1));
    }
  }
  Ok(())
}

// ─── Registry client ─────────────────────────────────────────────────────────

/// The server wraps a stack's manifest inside a `config` field (alongside
/// metadata like stars/owner); the CLI only needs the manifest itself.
#[derive(Deserialize)]
struct StackResponse {
  config: StackManifest,
}

#[derive(Serialize)]
struct PublishStackDto {
  url: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  visibility: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  repo_credential_id: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  organization_id: Option<String>,
}

/// A trimmed registry catalog row for `stack list`.
#[derive(Deserialize)]
pub struct CatalogStack {
  pub stack_id: String,
  pub name: String,
  #[serde(default)]
  pub description: String,
  #[serde(default)]
  pub star_count: i64,
  pub config: StackManifest,
}

#[derive(Deserialize)]
struct Paginated {
  data: Vec<CatalogStack>,
  total_pages: i64,
}

/// Fetches a stack's manifest from the registry (`GET /stack/{id}`).
pub async fn fetch_stack_manifest(ctx: &AppContext, stack_id: &str) -> Result<StackManifest> {
  let token = get_auth_user(&ctx.paths.auth).ok().map(|u| u.token);
  let mut req = ctx.client.get(format!("{}/stack/{stack_id}", ctx.backend_url));
  if let Some(token) = token {
    req = req.bearer_auth(token);
  }
  let res: StackResponse = req
    .send()
    .await
    .with_context(|| format!("Failed to fetch stack '{stack_id}' from registry"))?
    .error_for_status()
    .with_context(|| format!("Stack '{stack_id}' not found in registry"))?
    .json()
    .await
    .with_context(|| format!("Failed to parse stack '{stack_id}'"))?;
  validate(&res.config)?;
  Ok(res.config)
}

/// Fetches the full stack catalog (`GET /stack/all`), following pagination.
pub async fn fetch_stack_catalog(ctx: &AppContext) -> Result<Vec<CatalogStack>> {
  let token = get_auth_user(&ctx.paths.auth).ok().map(|u| u.token);
  let mut all = Vec::new();
  let mut page = 1;
  loop {
    let mut req = ctx
      .client
      .get(format!("{}/stack/all", ctx.backend_url))
      .query(&[("page", page), ("page_size", 100)]);
    if let Some(token) = &token {
      req = req.bearer_auth(token);
    }
    let res: Paginated = req
      .send()
      .await
      .context("Failed to connect to server for the stack catalog")?
      .error_for_status()
      .context("Server returned an error for the stack catalog")?
      .json()
      .await
      .context("Failed to parse the stack catalog")?;
    let total_pages = res.total_pages;
    all.extend(res.data);
    if page >= total_pages {
      break;
    }
    page += 1;
  }
  Ok(all)
}

pub async fn publish_stack(
  ctx: &AppContext,
  stack_url: &str,
  update: bool,
  visibility: Option<String>,
  credential_id: Option<String>,
  org_id: Option<String>,
) -> Result<()> {
  let user = get_auth_user(&ctx.paths.auth)?;
  let (method, verb) = if update {
    (ctx.client.patch(format!("{}/stack/", ctx.backend_url)), "Updating")
  } else {
    (ctx.client.post(format!("{}/stack/publish", ctx.backend_url)), "Publishing")
  };

  let sp = spinner(format!("{verb} stack to registry..."));
  let res = method
    .bearer_auth(user.token)
    .header("Content-Type", "application/json")
    .json(&PublishStackDto {
      url: stack_url.to_string(),
      visibility,
      repo_credential_id: credential_id,
      organization_id: org_id,
    })
    .send()
    .await
    .inspect_err(|_| sp.finish_and_clear())?
    .error_for_status()
    .inspect_err(|_| sp.finish_and_clear())?;
  sp.finish_and_clear();

  let body: serde_json::Value = res.json().await.unwrap_or_default();
  let message = body
    .get("message")
    .and_then(|m| m.as_str())
    .unwrap_or("Done");
  println!("✅ {message}");
  if let Some(id) = body.get("stack_id").and_then(|m| m.as_str()) {
    println!("   Stack: {id}");
  }
  Ok(())
}

/// Builds picker rows for the registry stack catalog, so stacks show up in
/// `anesis search` alongside templates and addons.
pub async fn stack_pick_items(ctx: &AppContext) -> Result<Vec<PickItem>> {
  let stacks = fetch_stack_catalog(ctx).await?;
  Ok(
    stacks
      .into_iter()
      .map(|s| {
        let meta = if s.star_count > 0 {
          format!(" · {}★", s.star_count)
        } else {
          String::new()
        };
        PickItem {
          kind: ItemKind::Stack,
          haystack: format!("{} {} {}", s.stack_id, s.name, s.description).to_lowercase(),
          id: s.stack_id,
          name: s.name,
          meta,
          description: s.description,
        }
      })
      .collect(),
  )
}

// ─── Local cache ─────────────────────────────────────────────────────────────

fn cached_path(ctx: &AppContext, stack_id: &str) -> std::path::PathBuf {
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

/// Prints installed stacks as a table, or the whole set as JSON.
pub fn print_installed_stacks(ctx: &AppContext, json: bool) -> Result<()> {
  let stacks = read_installed_stacks(ctx)?;
  if json {
    println!("{}", serde_json::to_string_pretty(&stacks)?);
    return Ok(());
  }
  if stacks.is_empty() {
    println!("No stacks installed yet. Install one with `anesis stack install <id>`.");
    return Ok(());
  }
  for s in &stacks {
    println!(
      "{}  {} {}",
      s.id.cyan().bold(),
      s.name,
      format!("({} + {} addons)", s.template, s.addons.len()).dimmed()
    );
  }
  Ok(())
}

/// Prints a single stack's composition (template + addon steps) or its JSON.
pub async fn stack_info(ctx: &AppContext, stack_id: &str, json: bool) -> Result<()> {
  // Prefer the registry (authoritative) but fall back to a cached copy offline.
  let manifest = match fetch_stack_manifest(ctx, stack_id).await {
    Ok(m) => m,
    Err(e) => {
      let cached = cached_path(ctx, stack_id);
      if cached.exists() {
        load_stack(&cached)?
      } else {
        return Err(e);
      }
    }
  };
  if json {
    println!("{}", serde_json::to_string_pretty(&manifest)?);
    return Ok(());
  }
  println!("{} {}", manifest.name.bold(), format!("({})", manifest.id).dimmed());
  if !manifest.description.is_empty() {
    println!("{}", manifest.description);
  }
  println!("\n{} {}", "template:".dimmed(), manifest.template.cyan());
  println!("{}", "addons:".dimmed());
  for a in &manifest.addons {
    println!("  {} {}", a.id.cyan(), a.command.dimmed());
  }
  println!(
    "\nScaffold it:  {}",
    format!("anesis new <dir> --stack {}", manifest.id).cyan()
  );
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_and_defaults_command() {
    let stack: StackManifest = serde_json::from_str(
      r#"{"schema_version":"1","id":"s","name":"S","template":"nest-express",
          "addons":[{"id":"nest-config"},{"id":"nest-prisma","command":"generate","inputs":{"db":"postgres"}}]}"#,
    )
    .unwrap();
    validate(&stack).unwrap();
    assert_eq!(stack.addons[0].command, "install");
    assert_eq!(stack.addons[1].command, "generate");
    assert_eq!(stack.addons[1].inputs.get("db").unwrap(), "postgres");
  }

  #[test]
  fn rejects_empty_template_and_addon_id() {
    let mut stack: StackManifest = serde_json::from_str(
      r#"{"schema_version":"1","id":"s","name":"S","template":"nest"}"#,
    )
    .unwrap();
    stack.template = " ".into();
    assert!(validate(&stack).is_err());
    stack.template = "nest".into();
    stack.addons.push(StackAddon { id: "".into(), command: "install".into(), inputs: HashMap::new() });
    assert!(validate(&stack).is_err());
  }
}
