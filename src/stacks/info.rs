use anyhow::Result;
use colored::Colorize;

use super::{
  cache::{cached_path, read_installed_stacks},
  manifest::load_stack,
  registry::fetch_stack_manifest,
};
use crate::context::AppContext;

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
