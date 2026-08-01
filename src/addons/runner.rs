use std::{
  collections::HashMap,
  fs,
  path::Path,
  sync::{Arc, Mutex},
};

use anyhow::{Context, Result, anyhow};
use colored::Colorize;
use inquire::{Confirm, Select, Text};

use crate::{
  context::{AppContext, CleanupState, CleanupTask},
  manifest::AnesisManifest,
  templates::generator::{to_camel_case, to_kebab_case, to_pascal_case, to_snake_case},
  utils::{
    picker::{ItemKind, PickItem, pick_one},
    ui::spinner,
  },
};

use super::{
  detect::detect_variant,
  install::{fetch_latest_version, install_addon, read_cached_manifest, record_addon_use},
  lock::{LockEntry, LockFile},
  manifest::{AddonCommand, InputDef, InputType},
  steps::{
    Rollback, append::execute_append, copy::execute_copy, create::execute_create,
    delete::execute_delete, inject::execute_inject, move_step::execute_move,
    packages::execute_packages, rename::execute_rename, replace::execute_replace, run::execute_run,
  },
};
use crate::addons::manifest::Step;

fn take_journal(journal: &Mutex<Vec<Rollback>>) -> Vec<Rollback> {
  std::mem::take(&mut *journal.lock().unwrap_or_else(|e| e.into_inner()))
}

struct ClearCleanupOnDrop<'a>(&'a CleanupState);

impl Drop for ClearCleanupOnDrop<'_> {
  fn drop(&mut self) {
    let mut guard = self.0.lock().unwrap_or_else(|e| e.into_inner());
    *guard = None;
  }
}

pub async fn run_addon_command(
  ctx: &AppContext,
  addon_id: &str,
  command_name: &str,
  project_root: &Path,
  presets: &HashMap<String, String>,
  non_interactive: bool,
  dry_run: bool,
) -> Result<()> {
  let non_interactive = non_interactive || dry_run;
  let addon_dir = ctx.paths.addons.join(addon_id);
  let cached = super::cache::get_cached_addon(&ctx.paths.addons, addon_id)?;

  let (manifest, update_check) = if let Some(cached) = cached.filter(|_| addon_dir.exists()) {
    let manifest = read_cached_manifest(&ctx.paths.addons, addon_id)?;
    let handle = tokio::spawn(super::install::check_addon_update(
      ctx.client.clone(),
      ctx.backend_url.clone(),
      ctx.paths.auth.clone(),
      addon_id.to_string(),
      cached.commit_sha,
    ));
    (manifest, Some(handle))
  } else {
    let sp = spinner(format!("Fetching addon '{addon_id}'..."));
    let install_result = install_addon(ctx, addon_id)
      .await
      .inspect_err(|_| sp.finish_and_clear())?;
    sp.finish_and_clear();
    if let Some(message) = install_result.message(addon_id) {
      println!("{message}");
    }
    (install_result.into_manifest(), None)
  };

  let mut lock = LockFile::load(project_root)?;

  for dep_id in &manifest.requires {
    if !lock.addons.iter().any(|e| &e.id == dep_id) {
      return Err(anyhow!(
        "Addon '{}' requires '{}' to be applied in this project first. Run: anesis use {} <command>",
        addon_id,
        dep_id,
        dep_id
      ));
    }
  }

  let detected_id = detect_variant(&manifest.detect, project_root);

  let variant = manifest
    .variants
    .iter()
    .find(|v| v.when.as_deref() == detected_id.as_deref())
    .or_else(|| manifest.variants.iter().find(|v| v.when.is_none()))
    .ok_or_else(|| anyhow!("No matching variant found for addon '{}'", addon_id))?;

  let command = variant
    .commands
    .iter()
    .find(|c| c.name == command_name)
    .ok_or_else(|| {
      anyhow!(
        "Command '{}' not found in addon '{}'",
        command_name,
        addon_id
      )
    })?;

  if !dry_run && command.once && lock.is_command_executed(addon_id, command_name) {
    if let Some(prompt_message) = rerun_prompt_message(
      command_name,
      lock.addon_version(addon_id),
      &manifest.version,
    ) {
      let rerun = if non_interactive {
        false
      } else {
        Confirm::new(&prompt_message).with_default(false).prompt()?
      };
      if !rerun {
        println!("Skipping command '{}'.", command_name);
        return Ok(());
      }
    } else {
      println!(
        "Command '{}' has already been executed, skipping.",
        command_name
      );
      return Ok(());
    }
  }

  for req_cmd in &command.requires_commands {
    if !lock.is_command_executed(addon_id, req_cmd) {
      return Err(anyhow!(
        "Command '{}' requires '{}' to be run first. Run: anesis use {} {}",
        command_name,
        req_cmd,
        addon_id,
        req_cmd
      ));
    }
  }

  let mut tera_ctx = tera::Context::new();

  let mut input_values: HashMap<String, String> = HashMap::new();
  collect_inputs(
    &manifest.inputs,
    presets,
    non_interactive,
    &mut input_values,
  )?;
  insert_with_derived(&mut tera_ctx, &input_values);

  let mut cmd_input_values: HashMap<String, String> = HashMap::new();
  collect_inputs(
    &command.inputs,
    presets,
    non_interactive,
    &mut cmd_input_values,
  )?;
  insert_with_derived(&mut tera_ctx, &cmd_input_values);

  if dry_run {
    print_dry_run_plan(
      addon_id,
      command_name,
      detected_id.as_deref(),
      &input_values,
      &cmd_input_values,
      &command.steps,
    );
    return Ok(());
  }

  if !confirm_addon_execution(addon_id, command_name, &command.steps, non_interactive)? {
    println!("Aborted. No changes were made.");
    return Ok(());
  }

  let addon_dir = ctx.paths.addons.join(addon_id);
  let total = command.steps.len();

  let journal: Arc<Mutex<Vec<Rollback>>> = Arc::new(Mutex::new(Vec::new()));
  {
    let mut guard = ctx.cleanup_state.lock().unwrap_or_else(|e| e.into_inner());
    *guard = Some(CleanupTask::PartialAddon {
      addon_id: addon_id.to_string(),
      project_root: project_root.to_path_buf(),
      journal: Arc::clone(&journal),
    });
  }
  let _cleanup_guard = ClearCleanupOnDrop(&ctx.cleanup_state);

  for (idx, step) in command.steps.iter().enumerate() {
    let label = step_label(step);
    println!("{} {}", format!("[{}/{}]", idx + 1, total).dimmed(), label);

    let result = match step {
      Step::Copy(s) => execute_copy(s, &addon_dir, project_root, &tera_ctx, non_interactive),
      Step::Create(s) => execute_create(s, project_root, &tera_ctx, non_interactive),
      Step::Inject(s) => execute_inject(s, project_root, &tera_ctx),
      Step::Replace(s) => execute_replace(s, project_root, &tera_ctx),
      Step::Append(s) => execute_append(s, project_root, &tera_ctx),
      Step::Delete(s) => execute_delete(s, project_root, &tera_ctx),
      Step::Rename(s) => execute_rename(s, project_root, &tera_ctx),
      Step::Move(s) => execute_move(s, project_root, &tera_ctx),
      Step::Packages(s) => execute_packages(s, project_root, non_interactive, ctx.allow_run),
      Step::Run(s) => execute_run(s, project_root, &tera_ctx, non_interactive, ctx.allow_run),
    }
    .with_context(|| format!("step {} ({}) failed", idx + 1, label));

    match result {
      Ok(rollbacks) => journal
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .extend(rollbacks),
      Err(err) => {
        eprintln!("{} {:#}", "✗".red().bold(), err);
        let choice = if non_interactive {
          "Rollback all changes"
        } else {
          Select::new(
            "How would you like to proceed?",
            vec!["Keep changes made so far", "Rollback all changes"],
          )
          .prompt()?
        };

        if choice == "Rollback all changes" {
          for rollback in take_journal(&journal).into_iter().rev() {
            let _ = apply_rollback(rollback, project_root);
          }
          println!("Rolled back all changes made by this command.");
        }

        return Err(err);
      }
    }
  }

  let completed_rollbacks = take_journal(&journal);
  {
    let mut guard = ctx.cleanup_state.lock().unwrap_or_else(|e| e.into_inner());
    *guard = None;
  }

  let variant_id = detected_id.unwrap_or_else(|| "universal".to_string());
  if let Some(existing) = lock.addons.iter_mut().find(|e| e.id == addon_id) {
    existing.version = manifest.version.clone();
    existing.variant = variant_id;
    existing.journal.extend(completed_rollbacks);
    existing.inputs.extend(
      input_values
        .iter()
        .chain(&cmd_input_values)
        .map(|(k, v)| (k.clone(), v.clone())),
    );
  } else {
    let mut inputs = input_values.clone();
    inputs.extend(cmd_input_values.iter().map(|(k, v)| (k.clone(), v.clone())));
    lock.addons.push(LockEntry {
      id: addon_id.to_string(),
      version: manifest.version.clone(),
      variant: variant_id,
      commands_executed: Vec::new(),
      journal: completed_rollbacks,
      inputs,
    });
  }
  lock.mark_command_executed(addon_id, command_name);
  lock.save(project_root)?;

  record_addon_use(ctx, addon_id).await;

  if let Err(err) = AnesisManifest::add_addon(addon_id, project_root) {
    eprintln!("Note: could not update anesis.json ({err}).");
  }
  println!("✓ Command '{}' completed successfully.", command_name);

  if let Some(handle) = update_check
    && matches!(handle.await, Ok(Some(_)))
  {
    let sp = spinner(format!(
      "A newer version of '{addon_id}' is available, updating..."
    ));
    let updated = install_addon(ctx, addon_id).await;
    sp.finish_and_clear();
    match updated {
      Ok(result) => {
        if let Some(message) = result.update_message(addon_id) {
          println!("{message} (will be used next time)");
        }
      }
      Err(err) => eprintln!("Note: could not update addon '{addon_id}' ({err})."),
    }
  }

  Ok(())
}

pub async fn list_addon_commands(
  ctx: &AppContext,
  addon_id: &str,
  project_root: &Path,
  presets: &HashMap<String, String>,
  non_interactive: bool,
  dry_run: bool,
) -> Result<()> {
  let addon_dir = ctx.paths.addons.join(addon_id);
  let cached = super::cache::get_cached_addon(&ctx.paths.addons, addon_id)?;
  let manifest = if cached.is_some() && addon_dir.exists() {
    read_cached_manifest(&ctx.paths.addons, addon_id)?
  } else {
    let sp = spinner(format!("Fetching addon '{addon_id}'..."));
    let result = install_addon(ctx, addon_id)
      .await
      .inspect_err(|_| sp.finish_and_clear())?;
    sp.finish_and_clear();
    result.into_manifest()
  };

  let detected_id = detect_variant(&manifest.detect, project_root);
  let matched = manifest
    .variants
    .iter()
    .find(|v| v.when.as_deref() == detected_id.as_deref())
    .or_else(|| manifest.variants.iter().find(|v| v.when.is_none()));
  let commands: Vec<&AddonCommand> = match matched {
    Some(variant) => variant.commands.iter().collect(),
    None => manifest.variants.iter().flat_map(|v| &v.commands).collect(),
  };

  if commands.is_empty() {
    println!("Addon '{addon_id}' has no commands available for this project.");
    return Ok(());
  }

  let items: Vec<PickItem> = commands
    .iter()
    .map(|c| PickItem {
      kind: ItemKind::Addon,
      id: c.name.clone(),
      name: c.name.clone(),
      meta: String::new(),
      description: c.description.clone(),
      haystack: format!("{} {}", c.name, c.description).to_lowercase(),
    })
    .collect();

  match pick_one(
    items,
    format!("Commands for {addon_id}"),
    false,
    String::new(),
  )
  .await?
  {
    Some((_, command_name, _)) => {
      run_addon_command(
        ctx,
        addon_id,
        &command_name,
        project_root,
        presets,
        non_interactive,
        dry_run,
      )
      .await
    }
    None => Ok(()),
  }
}

fn step_label(step: &Step) -> String {
  use crate::addons::manifest::Target;
  fn target(t: &Target) -> &str {
    match t {
      Target::File { file } => file,
      Target::Glob { glob } => glob,
    }
  }
  match step {
    Step::Copy(s) => format!("copy '{}' → '{}'", s.src, s.dest),
    Step::Create(s) => format!("create '{}'", s.path),
    Step::Inject(s) => format!("inject into '{}'", target(&s.target)),
    Step::Replace(s) => format!("replace in '{}'", target(&s.target)),
    Step::Append(s) => format!("append to '{}'", target(&s.target)),
    Step::Delete(s) => format!("delete '{}'", target(&s.target)),
    Step::Rename(s) => format!("rename '{}' → '{}'", s.from, s.to),
    Step::Move(s) => format!("move '{}' → '{}'", s.from, s.to),
    Step::Packages(s) => format!(
      "install {} package(s)",
      s.dependencies.len() + s.dev_dependencies.len()
    ),
    Step::Run(s) => format!("run '{}'", s.command),
  }
}

fn print_dry_run_plan(
  addon_id: &str,
  command_name: &str,
  variant: Option<&str>,
  addon_inputs: &HashMap<String, String>,
  cmd_inputs: &HashMap<String, String>,
  steps: &[Step],
) {
  println!(
    "{} {} {}",
    "Dry run:".bold(),
    addon_id.cyan(),
    command_name.cyan()
  );
  println!(
    "  {} {}",
    "variant:".dimmed(),
    variant.unwrap_or("universal")
  );

  let mut inputs: Vec<(&String, &String)> = addon_inputs.iter().chain(cmd_inputs.iter()).collect();
  inputs.sort_by(|a, b| a.0.cmp(b.0));
  if inputs.is_empty() {
    println!("  {} (none)", "inputs:".dimmed());
  } else {
    println!("  {}", "inputs:".dimmed());
    for (k, v) in inputs {
      println!("    {k} = {v}");
    }
  }

  println!("  {} {} step(s)", "steps:".dimmed(), steps.len());
  for (idx, step) in steps.iter().enumerate() {
    println!(
      "    {} {}",
      format!("[{}/{}]", idx + 1, steps.len()).dimmed(),
      step_label(step)
    );
  }
  println!("\nNo files were changed.");
}

fn confirm_addon_execution(
  addon_id: &str,
  command_name: &str,
  steps: &[Step],
  non_interactive: bool,
) -> Result<bool> {
  if non_interactive {
    return Ok(true);
  }
  let (mut writes, mut edits, mut removes) = (0usize, 0usize, 0usize);
  for step in steps {
    match step {
      Step::Create(_) | Step::Copy(_) => writes += 1,
      Step::Inject(_) | Step::Replace(_) | Step::Append(_) | Step::Packages(_) | Step::Run(_) => {
        edits += 1
      }
      Step::Delete(_) | Step::Rename(_) | Step::Move(_) => removes += 1,
    }
  }

  println!(
    "⚠ Addon '{addon_id}' command '{command_name}' will modify files in this project \
     ({writes} created/copied, {edits} edited, {removes} deleted/moved)."
  );
  println!(
    "  Addons run unsandboxed and can overwrite source files or 'package.json'. \
     Only run addons you trust."
  );

  Ok(Confirm::new("Proceed?").with_default(false).prompt()?)
}

fn rerun_prompt_message(
  command_name: &str,
  locked_version: Option<&str>,
  current_version: &str,
) -> Option<String> {
  let locked_version = locked_version.filter(|version| !version.is_empty())?;
  if locked_version == current_version {
    return None;
  }

  Some(format!(
    "Command '{}' was last run with v{} of this add-on. A new version (v{}) is available. Re-run it now?",
    command_name, locked_version, current_version
  ))
}

#[doc(hidden)]
pub fn rerun_prompt_message_for_tests(
  command_name: &str,
  locked_version: Option<&str>,
  current_version: &str,
) -> Option<String> {
  rerun_prompt_message(command_name, locked_version, current_version)
}

pub fn collect_inputs(
  inputs: &[InputDef],
  presets: &HashMap<String, String>,
  non_interactive: bool,
  map: &mut HashMap<String, String>,
) -> Result<()> {
  let mut missing: Vec<&str> = Vec::new();
  for input in inputs {
    if let Some(preset) = presets.get(&input.name) {
      map.insert(input.name.clone(), preset.clone());
      continue;
    }
    if non_interactive {
      match &input.default {
        Some(default) => {
          map.insert(input.name.clone(), default.clone());
        }
        None if input.required => missing.push(&input.name),
        None => {
          let fallback = match input.input_type {
            InputType::Boolean => "false".to_string(),
            _ => String::new(),
          };
          map.insert(input.name.clone(), fallback);
        }
      }
      continue;
    }
    let value = match input.input_type {
      InputType::Text => loop {
        let mut prompt = Text::new(&input.description);
        if let Some(ref default) = input.default {
          prompt = prompt.with_default(default);
        }
        let value = prompt.prompt()?;
        if input.required && value.trim().is_empty() {
          eprintln!("'{}' is required; please enter a value.", input.name);
          continue;
        }
        break value;
      },
      InputType::Boolean => {
        let default = input
          .default
          .as_deref()
          .map(|d| d == "true")
          .unwrap_or(false);
        Confirm::new(&input.description)
          .with_default(default)
          .prompt()?
          .to_string()
      }
      InputType::Select => Select::new(&input.description, input.options.clone())
        .prompt()?
        .to_string(),
    };
    map.insert(input.name.clone(), value);
  }
  if !missing.is_empty() {
    return Err(anyhow!(
      "Missing required input(s) in non-interactive mode: {}. Provide them with --input NAME=VALUE.",
      missing.join(", ")
    ));
  }
  Ok(())
}

fn insert_with_derived(ctx: &mut tera::Context, map: &HashMap<String, String>) {
  for (k, v) in map {
    ctx.insert(k.as_str(), v);
    ctx.insert(format!("{k}_pascal"), &to_pascal_case(v));
    ctx.insert(format!("{k}_camel"), &to_camel_case(v));
    ctx.insert(format!("{k}_kebab"), &to_kebab_case(v));
    ctx.insert(format!("{k}_snake"), &to_snake_case(v));
  }
}

fn prune_empty_dirs(start: Option<&Path>, project_root: &Path) {
  let mut dir = start;
  while let Some(d) = dir {
    if d == project_root || !d.starts_with(project_root) || fs::remove_dir(d).is_err() {
      break;
    }
    dir = d.parent();
  }
}

#[doc(hidden)]
pub fn prune_empty_dirs_for_tests(start: Option<&Path>, project_root: &Path) {
  prune_empty_dirs(start, project_root);
}

pub fn undo_addon(addon_id: &str, project_root: &Path, non_interactive: bool) -> Result<()> {
  let mut lock = LockFile::load(project_root)?;
  let entry = lock
    .addons
    .iter()
    .find(|e| e.id == addon_id)
    .filter(|e| !e.journal.is_empty())
    .ok_or_else(|| {
      anyhow!("Addon '{addon_id}' has no undoable changes recorded in this project.")
    })?;

  let mut conflicts: Vec<String> = Vec::new();
  for rollback in &entry.journal {
    match rollback {
      Rollback::DeleteCreatedFile { path } if !path.exists() => {
        conflicts.push(format!("{} (already deleted)", path.display()));
      }
      Rollback::RestoreFile { path, .. } if !path.exists() => {
        conflicts.push(format!("{} (missing)", path.display()));
      }
      Rollback::RenameFile { to, .. } if !to.exists() => {
        conflicts.push(format!("{} (missing)", to.display()));
      }
      _ => {}
    }
  }

  if !conflicts.is_empty() {
    eprintln!(
      "{} some files changed since '{addon_id}' was applied:",
      "⚠".yellow().bold()
    );
    for c in &conflicts {
      eprintln!("  {c}");
    }
  }

  if !non_interactive
    && !Confirm::new(&format!("Undo addon '{addon_id}'?"))
      .with_default(conflicts.is_empty())
      .prompt()?
  {
    println!("Aborted. No changes were made.");
    return Ok(());
  }

  let entry = lock.addons.iter_mut().find(|e| e.id == addon_id).unwrap();
  let journal = std::mem::take(&mut entry.journal);
  for rollback in journal.into_iter().rev() {
    apply_rollback(rollback, project_root)?;
  }

  lock.remove_addon(addon_id);
  lock.save(project_root)?;

  if let Err(err) = AnesisManifest::remove_addon(addon_id, project_root) {
    eprintln!("Note: could not update anesis.json ({err}).");
  }

  println!("✓ Reverted addon '{addon_id}'.");
  Ok(())
}

fn is_newer(latest: &str, current: &str) -> bool {
  match (
    semver::Version::parse(latest),
    semver::Version::parse(current),
  ) {
    (Ok(l), Ok(c)) => l > c,
    _ => latest != current,
  }
}

#[doc(hidden)]
pub fn is_newer_for_tests(latest: &str, current: &str) -> bool {
  is_newer(latest, current)
}

#[derive(serde::Serialize)]
pub struct OutdatedEntry {
  pub id: String,
  pub current: String,
  pub latest: Option<String>,
  pub outdated: bool,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub error: Option<String>,
}

pub async fn outdated(ctx: &AppContext, project_root: &Path, json: bool) -> Result<()> {
  let lock = LockFile::load(project_root)?;

  if lock.addons.is_empty() {
    if json {
      println!("[]");
    } else {
      println!("No addons applied in this project.");
    }
    return Ok(());
  }

  let mut entries = Vec::with_capacity(lock.addons.len());
  for entry in &lock.addons {
    entries.push(match fetch_latest_version(ctx, &entry.id).await {
      Ok(latest) => OutdatedEntry {
        id: entry.id.clone(),
        current: entry.version.clone(),
        outdated: is_newer(&latest, &entry.version),
        latest: Some(latest),
        error: None,
      },
      Err(err) => OutdatedEntry {
        id: entry.id.clone(),
        current: entry.version.clone(),
        latest: None,
        outdated: false,
        error: Some(format!("{err:#}")),
      },
    });
  }

  if json {
    println!("{}", serde_json::to_string_pretty(&entries)?);
    return Ok(());
  }

  let mut any = false;
  for entry in &entries {
    match (&entry.latest, &entry.error) {
      (Some(latest), _) if entry.outdated => {
        any = true;
        println!(
          "  {} v{} → v{}",
          entry.id.cyan(),
          entry.current,
          latest.green()
        );
      }
      (_, Some(err)) => eprintln!("  {} (could not check: {err})", entry.id.dimmed()),
      _ => {}
    }
  }

  if any {
    println!("\nRun `anesis update <addon_id>` to upgrade.");
  } else {
    println!("All addons are up to date.");
  }
  Ok(())
}

pub async fn update_addon(
  ctx: &AppContext,
  addon_id: &str,
  project_root: &Path,
  non_interactive: bool,
) -> Result<()> {
  let (current, commands, saved_inputs, had_journal) = {
    let lock = LockFile::load(project_root)?;
    let entry = lock
      .addons
      .iter()
      .find(|e| e.id == addon_id)
      .ok_or_else(|| anyhow!("Addon '{addon_id}' is not applied in this project."))?;
    (
      entry.version.clone(),
      entry.commands_executed.clone(),
      entry.inputs.clone(),
      !entry.journal.is_empty(),
    )
  };

  let latest = fetch_latest_version(ctx, addon_id).await?;
  if !is_newer(&latest, &current) {
    println!("Addon '{addon_id}' is already up to date (v{current}).");
    return Ok(());
  }

  println!("Updating '{addon_id}' v{current} → v{latest}...");

  if had_journal {
    undo_addon(addon_id, project_root, true)?;
  } else {
    let mut lock = LockFile::load(project_root)?;
    lock.remove_addon(addon_id);
    lock.save(project_root)?;
    let _ = AnesisManifest::remove_addon(addon_id, project_root);
  }
  install_addon(ctx, addon_id).await?;

  for cmd in &commands {
    run_addon_command(
      ctx,
      addon_id,
      cmd,
      project_root,
      &saved_inputs,
      non_interactive,
      false,
    )
    .await?;
  }

  println!("✓ Updated '{addon_id}' to v{latest}.");
  Ok(())
}

pub fn apply_rollback(rollback: Rollback, project_root: &Path) -> Result<()> {
  match rollback {
    Rollback::DeleteCreatedFile { path } => {
      let _ = std::fs::remove_file(&path);
      prune_empty_dirs(path.parent(), project_root);
    }
    Rollback::RestoreFile { path, original } => {
      std::fs::write(path, original)?;
    }
    Rollback::RenameFile { from, to } => {
      std::fs::rename(&from, to)?;
      prune_empty_dirs(from.parent(), project_root);
    }
    Rollback::IrreversibleRun { command } => {
      eprintln!(
        "{} could not undo shell command '{}' — its effects remain.",
        "⚠".yellow().bold(),
        command
      );
    }
  }
  Ok(())
}
