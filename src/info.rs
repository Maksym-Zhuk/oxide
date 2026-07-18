use std::{fs, path::Path};

use colored::Colorize;

use crate::{auth::token::get_auth_user, context::AppContext};

pub fn print_info(ctx: &AppContext) {
  println!(
    "{} {}",
    "anesis".bold().cyan(),
    format!("v{}", env!("CARGO_PKG_VERSION")).dimmed()
  );
  println!();

  match get_auth_user(&ctx.paths.auth) {
    Ok(user) => println!(
      "{}  {} {}",
      "Account:".bold(),
      "✓".green(),
      format!("logged in as {}", user.name).green()
    ),
    Err(_) => println!(
      "{}  {} {}",
      "Account:".bold(),
      "✗".yellow(),
      "not logged in (run: anesis login)".yellow()
    ),
  }
  println!("{}  {}", "Backend:".bold(), ctx.backend_url.dimmed());
  println!();

  println!("{}", "Paths:".bold());
  print_path("home", &ctx.paths.home);
  print_path("templates cache", &ctx.paths.templates);
  print_path("addons cache", &ctx.paths.addons);
  print_path("auth", &ctx.paths.auth);
  println!();

  let templates = count_array(
    &ctx.paths.templates.join("anesis-templates.json"),
    "templates",
  );
  let addons = count_array(&ctx.paths.addons_index, "addons");
  println!(
    "{} {} template(s), {} addon(s)",
    "Installed:".bold(),
    templates.to_string().cyan(),
    addons.to_string().cyan()
  );
}

pub fn info_json(ctx: &AppContext) -> serde_json::Value {
  let user = get_auth_user(&ctx.paths.auth).ok();
  serde_json::json!({
    "version": env!("CARGO_PKG_VERSION"),
    "backend": ctx.backend_url,
    "logged_in": user.is_some(),
    "account": user.map(|u| u.name),
    "paths": {
      "home": ctx.paths.home,
      "templates_cache": ctx.paths.templates,
      "addons_cache": ctx.paths.addons,
      "auth": ctx.paths.auth,
    },
    "installed": {
      "templates": count_array(&ctx.paths.templates.join("anesis-templates.json"), "templates"),
      "addons": count_array(&ctx.paths.addons_index, "addons"),
    },
  })
}

fn print_path(label: &str, path: &Path) {
  println!("  {:<16} {}", format!("{label}:").dimmed(), path.display());
}

fn count_array(path: &Path, field: &str) -> usize {
  fs::read_to_string(path)
    .ok()
    .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
    .and_then(|value| value.get(field)?.as_array().map(Vec::len))
    .unwrap_or(0)
}
