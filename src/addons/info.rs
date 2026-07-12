use anyhow::Result;
use colored::Colorize;

use crate::{context::AppContext, utils::ui::spinner};

use super::{
  install::{install_addon, read_cached_manifest},
  manifest::{AddonManifest, InputDef, InputType, Step, Target},
};

/// Prints an addon's manifest — description, version, variants and each
/// command's inputs and steps. Reads from the local cache when the addon is
/// installed, otherwise fetches it from the registry (mirrors
/// `runner::list_addon_commands`). `--json` dumps the raw manifest.
pub async fn addon_info(ctx: &AppContext, addon_id: &str, json: bool) -> Result<()> {
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

  if json {
    println!("{}", serde_json::to_string_pretty(&manifest)?);
    return Ok(());
  }

  print_addon_info(&manifest);
  Ok(())
}

fn print_addon_info(m: &AddonManifest) {
  println!("{} {}", m.name.bold(), format!("({})", m.id).dimmed());
  println!(
    "{} {} {}",
    format!("v{}", m.version).cyan(),
    "by".dimmed(),
    m.author
  );
  if !m.description.is_empty() {
    println!("{}", m.description);
  }
  if !m.requires.is_empty() {
    println!("{} {}", "requires:".dimmed(), m.requires.join(", "));
  }

  for variant in &m.variants {
    let when = variant.when.as_deref().unwrap_or("universal");
    println!("\n{} {}", "variant".magenta().bold(), when.cyan());
    for cmd in &variant.commands {
      let once = if cmd.once {
        " (once)".dimmed().to_string()
      } else {
        String::new()
      };
      print!("  {} {}{}", "▸".magenta(), cmd.name.bold(), once);
      if cmd.description.is_empty() {
        println!();
      } else {
        println!(" {} {}", "—".dimmed(), cmd.description);
      }
      if !cmd.inputs.is_empty() {
        println!("    {}", "inputs:".dimmed());
        for input in &cmd.inputs {
          println!("      {}", input_line(input));
        }
      }
      if !cmd.steps.is_empty() {
        let total = cmd.steps.len();
        println!("    {}", "steps:".dimmed());
        for (idx, step) in cmd.steps.iter().enumerate() {
          println!(
            "      {} {}",
            format!("[{}/{}]", idx + 1, total).dimmed(),
            step_label(step)
          );
        }
      }
    }
  }
}

fn input_line(input: &InputDef) -> String {
  let ty = match input.input_type {
    InputType::Text => "text",
    InputType::Boolean => "boolean",
    InputType::Select => "select",
  };
  let mut extra = Vec::new();
  if input.required {
    extra.push("required".to_string());
  }
  if let Some(default) = &input.default {
    extra.push(format!("default: {default}"));
  }
  if !input.options.is_empty() {
    extra.push(format!("options: {}", input.options.join("/")));
  }
  let suffix = if extra.is_empty() {
    String::new()
  } else {
    format!(" ({})", extra.join(", "))
  };
  format!("{}: {}{}", input.name.cyan(), ty, suffix.dimmed())
}

/// Concise one-line summary of a step (type + target/path). Kept local rather
/// than reusing `runner::step_label` (private) to avoid widening its API.
fn step_label(step: &Step) -> String {
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
