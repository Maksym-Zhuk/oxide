use anesis::addons::catalog::{addon_pick_items, fetch_addon_catalog};
use anesis::{
  addons,
  auth::{account::print_user_info, login::login, logout::logout},
  cli::{
    self,
    commands::{AddonCommands, Commands, StackCommands, TemplateCommands},
  },
  completions, config,
  context::AppContext,
  info::print_info,
  stacks::registry::fetch_stack_catalog,
  templates::{
    cache::{get_installed_templates, remove_template_from_cache},
    catalog::{fetch_catalog, template_pick_items},
    generator::{extract_template, overwritten_paths},
    install::{InstallResult, install_template, record_template_use},
    link::link_template,
    loader::get_files,
    publish::publish,
    republish::republish,
  },
  upgrade::{check_cli_version_cached, render_upgrade_notice, upgrade_cli},
  utils::{
    errors::{exit_code_for, print_error},
    picker::{self, ItemKind, PickItem, pick_one},
    ui::spinner,
    validate::{is_valid_github_repo_url, validate_project_name, validate_template_name},
  },
};
use anyhow::Result;
use colored::Colorize;
use inquire::Confirm;

fn install_panic_hook() {
  let default_hook = std::panic::take_hook();

  std::panic::set_hook(Box::new(move |info| {
    picker::restore_terminal();

    if std::env::var("ANESIS_DEBUG").is_ok() {
      default_hook(info);
      return;
    }

    let location = info
      .location()
      .map(|l| format!("{}:{}", l.file(), l.line()))
      .unwrap_or_else(|| "unknown location".to_string());

    let message = info
      .payload()
      .downcast_ref::<&str>()
      .map(|s| (*s).to_string())
      .or_else(|| info.payload().downcast_ref::<String>().cloned())
      .unwrap_or_else(|| "unknown cause".to_string());

    eprintln!();
    eprintln!("{} anesis crashed. This is a bug.", "error:".red().bold());
    eprintln!();
    eprintln!("  {message}");
    eprintln!("  at {location}");
    eprintln!(
      "  anesis {} on {}",
      env!("CARGO_PKG_VERSION"),
      std::env::consts::OS
    );
    eprintln!();
    eprintln!(
      "  {} https://github.com/anesis-dev/anesis-cli/issues/new",
      "report it:".cyan().bold()
    );
    eprintln!("  Re-run with ANESIS_DEBUG=1 for a full backtrace to attach.");
  }));
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
  install_panic_hook();
  config::init_env();
  completions::complete_env();
  if let Err(err) = run().await {
    print_error(&err);
    std::process::exit(exit_code_for(&err));
  }
}

async fn run() -> Result<()> {
  let cli = cli::parse();

  config::init_logging(cli.verbose);
  if cli.no_color {
    colored::control::set_override(false);
  }
  anesis::utils::ui::set_quiet(cli.quiet);

  let ctx = config::build_app_context()?.with_cli_flags(cli.no_telemetry, cli.allow_run);

  let json_mode = matches!(
    &cli.command,
    Commands::Account { json: true }
      | Commands::Info { json: true }
      | Commands::Status { json: true }
      | Commands::Search { json: true, .. }
      | Commands::Outdated { json: true }
      | Commands::Template {
        command: TemplateCommands::List { json: true } | TemplateCommands::Info { json: true, .. }
      }
      | Commands::Addon {
        command: AddonCommands::List { json: true } | AddonCommands::Info { json: true, .. }
      }
      | Commands::Stack {
        command: StackCommands::List { json: true } | StackCommands::Info { json: true, .. }
      }
  );
  let skip_version_notice = json_mode
    || cli.quiet
    || matches!(
      &cli.command,
      Commands::Upgrade | Commands::Completions { .. } | Commands::Man { .. }
    );
  let version_check_handle = if skip_version_notice {
    None
  } else {
    let client = ctx.client.clone();
    let version_check_path = ctx.paths.version_check.clone();
    Some(tokio::spawn(async move {
      check_cli_version_cached(&client, &version_check_path).await
    }))
  };

  match cli.command {
    Commands::New {
      name,
      template_name,
      stack,
      installed,
      yes,
      input,
    } => {
      validate_project_name(&name)?;
      let inputs = parse_inputs(&input)?;
      if let Some(stack_ref) = stack {
        let stack = anesis::stacks::cache::resolve_stack(&ctx, &stack_ref).await?;
        apply_stack(&ctx, &name, &stack, yes, &inputs).await?;
      } else {
        let template_name = match template_name {
          Some(template_name) => template_name,
          None => choose_template(&ctx, installed, "Select a template").await?,
        };
        validate_template_name(&template_name)?;
        create_new_project(&ctx, &name, &template_name, yes, &inputs).await?;
      }
    }
    Commands::Template { command } => match command {
      TemplateCommands::Install { template_name } => {
        let template_name = match template_name {
          Some(name) => name,
          None => choose_template(&ctx, false, "Select a template to install").await?,
        };
        validate_template_name(&template_name)?;
        let install_result = install_template(&ctx, &template_name).await?;
        match install_result {
          InstallResult::UpToDate => {
            println!("{}", InstallResult::up_to_date_message(&template_name));
          }
          _ => {
            if let Some(message) = install_result.message(&template_name) {
              println!("{message}");
            }
          }
        }
      }
      TemplateCommands::Link { path, force } => {
        let source = std::path::PathBuf::from(path.as_deref().unwrap_or("."));
        match link_template(&ctx, &source, force)? {
          Some(name) => {
            println!("✅ Template '{name}' cached locally.");
            println!("   Try it:  {}", format!("anesis new <dir> {name}").cyan());
          }
          None => println!("Aborted. The cached template was left unchanged."),
        }
      }
      TemplateCommands::List { json } => {
        if json {
          let templates = anesis::templates::cache::read_installed_templates(&ctx.paths.templates)?;
          println!("{}", serde_json::to_string_pretty(&templates)?);
        } else {
          get_installed_templates(&ctx.paths.templates)?;
        }
      }
      TemplateCommands::Info {
        template_name,
        json,
      } => {
        anesis::templates::info::template_info(&ctx, &template_name, json).await?;
      }
      TemplateCommands::Remove { template_name } => {
        remove_template_from_cache(&ctx.paths.templates, &template_name)?;
      }
      TemplateCommands::Publish {
        template_url,
        visibility,
        credential_id,
        org_id,
      } => {
        is_valid_github_repo_url(&template_url)?;
        publish(&ctx, &template_url, visibility, credential_id, org_id).await?;
      }
      TemplateCommands::Republish {
        template_url,
        visibility,
        credential_id,
        org_id,
      } => {
        is_valid_github_repo_url(&template_url)?;
        republish(&ctx, &template_url, visibility, credential_id, org_id).await?;
      }
    },
    Commands::Login => {
      login(&ctx.paths.auth, &ctx.backend_url, &ctx.frontend_url).await?;
    }
    Commands::Logout => {
      logout(&ctx.paths.auth)?;
    }
    Commands::Account { json } => {
      if json {
        let user = anesis::auth::account::get_user_info(&ctx).await?;
        println!("{}", serde_json::to_string_pretty(&user)?);
      } else {
        print_user_info(&ctx).await?;
      }
    }
    Commands::Addon { command } => match command {
      AddonCommands::Install { addon_id } => {
        let addon_id = match addon_id {
          Some(id) => id,
          None => choose_addon(&ctx, false, "Select an addon to install").await?,
        };
        let install_result = addons::install::install_addon(&ctx, &addon_id).await?;
        match &install_result {
          addons::install::AddonInstallResult::UpToDate(_) => {
            println!(
              "{}",
              addons::install::AddonInstallResult::up_to_date_message(&addon_id)
            );
          }
          _ => {
            if let Some(message) = install_result.message(&addon_id) {
              println!("{message}");
            }
          }
        }
      }
      AddonCommands::Link { path, force } => {
        let source = std::path::PathBuf::from(path.as_deref().unwrap_or("."));
        match addons::link::link_addon(&ctx, &source, force)? {
          Some(id) => {
            println!("✅ Addon '{id}' cached locally.");
            println!(
              "   Try it:  {}",
              format!("anesis use {id} <command>").cyan()
            );
          }
          None => println!("Aborted. The cached addon was left unchanged."),
        }
      }
      AddonCommands::List { json } => {
        if json {
          let cache = addons::cache::read_cache(&ctx.paths.addons)?;
          println!("{}", serde_json::to_string_pretty(&cache.addons)?);
        } else {
          addons::cache::get_installed_addons(&ctx.paths.addons)?;
        }
      }
      AddonCommands::Info { addon_id, json } => {
        addons::info::addon_info(&ctx, &addon_id, json).await?;
      }
      AddonCommands::Test {
        addon_id,
        command,
        project,
      } => {
        addons::test::test_addon(&ctx, &addon_id, &command, project).await?;
      }
      AddonCommands::Remove { addon_id } => {
        addons::cache::remove_addon_from_cache(&ctx.paths.addons, &addon_id)?;
      }
      AddonCommands::Publish {
        addon_url,
        visibility,
        credential_id,
        org_id,
      } => {
        is_valid_github_repo_url(&addon_url)?;
        addons::publish::publish_addon(&ctx, &addon_url, visibility, credential_id, org_id).await?;
      }
      AddonCommands::Republish {
        addon_url,
        visibility,
        credential_id,
        org_id,
      } => {
        is_valid_github_repo_url(&addon_url)?;
        addons::republish::republish_addon(&ctx, &addon_url, visibility, credential_id, org_id)
          .await?;
      }
    },
    Commands::Stack { command } => match command {
      StackCommands::Install { stack_id } => {
        anesis::stacks::cache::install_stack(&ctx, &stack_id).await?;
        println!("✅ Stack '{stack_id}' installed.");
        println!(
          "   Scaffold it:  {}",
          format!("anesis new <dir> --stack {stack_id}").cyan()
        );
      }
      StackCommands::Link { path, force } => {
        let source = std::path::PathBuf::from(path.as_deref().unwrap_or("."));
        match anesis::stacks::link::link_stack(&ctx, &source, force)? {
          Some(id) => {
            println!("✅ Stack '{id}' cached locally.");
            println!(
              "   Try it:  {}",
              format!("anesis new <dir> --stack {id}").cyan()
            );
          }
          None => println!("Aborted. The cached stack was left unchanged."),
        }
      }
      StackCommands::List { json } => {
        anesis::stacks::info::print_installed_stacks(&ctx, json)?;
      }
      StackCommands::Info { stack_id, json } => {
        anesis::stacks::info::stack_info(&ctx, &stack_id, json).await?;
      }
      StackCommands::Remove { stack_id } => {
        anesis::stacks::cache::remove_cached_stack(&ctx, &stack_id)?;
      }
      StackCommands::Publish {
        stack_url,
        visibility,
        credential_id,
        org_id,
      } => {
        is_valid_github_repo_url(&stack_url)?;
        anesis::stacks::publish::publish_stack(
          &ctx,
          &stack_url,
          false,
          visibility,
          credential_id,
          org_id,
        )
        .await?;
      }
      StackCommands::Republish {
        stack_url,
        visibility,
        credential_id,
        org_id,
      } => {
        is_valid_github_repo_url(&stack_url)?;
        anesis::stacks::publish::publish_stack(
          &ctx,
          &stack_url,
          true,
          visibility,
          credential_id,
          org_id,
        )
        .await?;
      }
    },
    Commands::Use {
      addon_id,
      command,
      installed,
      yes,
      input,
      dry_run,
    } => {
      let project_root = std::env::current_dir()?;
      let presets = parse_inputs(&input)?;
      let addon_id = match addon_id {
        Some(id) => id,
        None => choose_addon(&ctx, installed, "Select an addon").await?,
      };
      match command {
        Some(command_name) => {
          addons::runner::run_addon_command(
            &ctx,
            &addon_id,
            &command_name,
            &project_root,
            &presets,
            yes,
            dry_run,
          )
          .await?;
        }
        None => {
          addons::runner::list_addon_commands(
            &ctx,
            &addon_id,
            &project_root,
            &presets,
            yes,
            dry_run,
          )
          .await?;
        }
      }
    }
    Commands::Undo { addon_id, yes } => {
      let project_root = std::env::current_dir()?;
      addons::runner::undo_addon(&addon_id, &project_root, yes)?;
    }
    Commands::Outdated { json } => {
      let project_root = std::env::current_dir()?;
      addons::runner::outdated(&ctx, &project_root, json).await?;
    }
    Commands::Update { addon_id, yes } => {
      let project_root = std::env::current_dir()?;
      addons::runner::update_addon(&ctx, &addon_id, &project_root, yes).await?;
    }
    Commands::Upgrade => {
      upgrade_cli(&ctx).await?;
    }
    Commands::Mcp => {
      anesis::mcp::run_mcp()?;
    }
    Commands::Man { dir } => {
      let dir = std::path::PathBuf::from(dir);
      let pages = anesis::man::generate(&dir)?;
      println!("Wrote {} man pages to {}", pages.len(), dir.display());
    }
    Commands::Completions { shell, print } => {
      if print {
        completions::print_completions(shell)?;
      } else {
        completions::install_completions(shell)?;
      }
    }
    Commands::Search { query, json } => {
      let sp = spinner("Loading registry...");
      let (templates, addons, stacks) = tokio::join!(
        fetch_catalog(&ctx),
        fetch_addon_catalog(&ctx),
        fetch_stack_catalog(&ctx)
      );
      sp.finish_and_clear();

      let mut items: Vec<PickItem> = templates?.iter().map(|t| t.to_pick_item()).collect();
      items.extend(addons?.iter().map(|a| a.to_pick_item()));
      items.extend(stacks.unwrap_or_default().iter().map(|s| s.to_pick_item()));

      if json {
        let needle = query.unwrap_or_default().to_lowercase();
        let results: Vec<serde_json::Value> = items
          .iter()
          .filter(|i| needle.is_empty() || i.haystack.contains(&needle))
          .map(|i| {
            serde_json::json!({
              "kind": match i.kind {
                ItemKind::Template => "template",
                ItemKind::Addon => "addon",
                ItemKind::Stack => "stack",
              },
              "id": i.id,
              "name": i.name,
              "description": i.description,
            })
          })
          .collect();
        println!("{}", serde_json::to_string_pretty(&results)?);
        return Ok(());
      }

      if items.is_empty() {
        println!("The registry is empty.");
      } else {
        let seed = query.unwrap_or_default();
        match pick_one(items, "Search the registry", true, seed).await? {
          Some((ItemKind::Template, id, name)) => {
            println!("{} {}", "template".green().bold(), name);
            println!(
              "  scaffold it:  {}",
              format!("anesis new <dir> {id}").cyan()
            );
          }
          Some((ItemKind::Addon, id, name)) => {
            println!("{} {}", "addon".magenta().bold(), name);
            println!(
              "  install:  {}",
              format!("anesis addon install {id}").cyan()
            );
            println!(
              "  run:      {}",
              format!("anesis use {id} <command>").cyan()
            );
          }
          Some((ItemKind::Stack, id, name)) => {
            println!("{} {}", "stack".blue().bold(), name);
            println!(
              "  scaffold it:  {}",
              format!("anesis new <dir> --stack {id}").cyan()
            );
          }
          None => {}
        }
      }
    }
    Commands::Info { json } => {
      if json {
        println!(
          "{}",
          serde_json::to_string_pretty(&anesis::info::info_json(&ctx))?
        );
      } else {
        print_info(&ctx);
      }
    }
    Commands::Status { json } => {
      let project_root = std::env::current_dir()?;
      if json {
        println!(
          "{}",
          serde_json::to_string_pretty(&anesis::status::status_json(&project_root)?)?
        );
      } else {
        anesis::status::print_status(&project_root)?;
      }
    }
  }

  if let Some(version_check_handle) = version_check_handle
    && let Ok(Ok(Some(latest_version))) = version_check_handle.await
  {
    println!("{}", render_upgrade_notice(&latest_version));
  }

  Ok(())
}

async fn choose_template(ctx: &AppContext, installed: bool, title: &'static str) -> Result<String> {
  let items = template_pick_items(ctx, installed).await?;
  if items.is_empty() {
    anyhow::bail!(if installed {
      "No templates are installed yet"
    } else {
      "No templates are available yet"
    });
  }
  match pick_one(items, title, false, String::new()).await? {
    Some((_, id, _)) => Ok(id),
    None => anyhow::bail!("Selection cancelled"),
  }
}

async fn choose_addon(ctx: &AppContext, installed: bool, title: &'static str) -> Result<String> {
  let items = addon_pick_items(ctx, installed).await?;
  if items.is_empty() {
    anyhow::bail!(if installed {
      "No addons are installed yet"
    } else {
      "No addons are available yet"
    });
  }
  match pick_one(items, title, false, String::new()).await? {
    Some((_, id, _)) => Ok(id),
    None => anyhow::bail!("Selection cancelled"),
  }
}

fn parse_inputs(pairs: &[String]) -> Result<std::collections::HashMap<String, String>> {
  let mut map = std::collections::HashMap::new();
  for pair in pairs {
    let (name, value) = pair
      .split_once('=')
      .ok_or_else(|| anyhow::anyhow!("Invalid --input '{pair}'; expected NAME=VALUE"))?;
    map.insert(name.to_string(), value.to_string());
  }
  Ok(map)
}

async fn apply_stack(
  ctx: &AppContext,
  project_name: &str,
  stack: &anesis::stacks::manifest::StackManifest,
  yes: bool,
  inputs: &std::collections::HashMap<String, String>,
) -> Result<()> {
  println!("Creating '{project_name}' from stack '{}'...", stack.name);
  create_new_project(ctx, project_name, &stack.template, yes, inputs).await?;

  let project_root = if project_name == "." {
    std::env::current_dir()?
  } else {
    std::env::current_dir()?.join(project_name)
  };

  let total = stack.addons.len();
  for (idx, addon) in stack.addons.iter().enumerate() {
    println!(
      "\n{} addon {}",
      format!("[{}/{}]", idx + 1, total).dimmed(),
      addon.id.cyan()
    );
    addons::runner::run_addon_command(
      ctx,
      &addon.id,
      &addon.command,
      &project_root,
      &addon.inputs,
      yes,
      false,
    )
    .await
    .map_err(|err| {
      anyhow::anyhow!(
        "stack '{}' failed while applying addon '{}': {err:#}",
        stack.id,
        addon.id
      )
    })?;
  }

  println!("\n✅ Stack '{}' applied.", stack.name);
  Ok(())
}

async fn create_new_project(
  ctx: &AppContext,
  project_name: &str,
  template_name: &str,
  yes: bool,
  presets: &std::collections::HashMap<String, String>,
) -> Result<()> {
  let files = get_files(ctx, template_name).await?;

  let mut inputs = std::collections::HashMap::new();
  let mut excluded = std::collections::HashSet::new();
  if let Some(manifest) = anesis::templates::generator::parse_template_manifest(&files) {
    anesis::compat::check_anesis_version(template_name, &manifest.anesis_version)?;
    addons::runner::collect_inputs(&manifest.inputs, presets, yes, &mut inputs)?;
    excluded = anesis::templates::generator::excluded_paths(&manifest.exclude, &inputs);
  }

  let overwrites = overwritten_paths(&files, project_name, &excluded)?;
  if !overwrites.is_empty() && !yes {
    println!(
      "⚠ Generating here will overwrite {} existing file(s):",
      overwrites.len()
    );
    for path in overwrites.iter().take(20) {
      println!("  {}", path.display());
    }
    if overwrites.len() > 20 {
      println!("  ...and {} more", overwrites.len() - 20);
    }
    if !Confirm::new("Continue and overwrite these files?")
      .with_default(false)
      .prompt()?
    {
      println!("Aborted. No changes were made.");
      return Ok(());
    }
  }

  if project_name != "." {
    println!("Generating project '{project_name}' from template '{template_name}'...");
  } else {
    println!("Generating project from template '{template_name}'...");
  }

  extract_template(&files, project_name, ctx, &inputs, &excluded)?;

  let sp = spinner("Finishing up...");
  record_template_use(ctx, template_name).await;
  sp.finish_and_clear();

  if project_name != "." {
    println!("✅ Project '{project_name}' created successfully!");
    println!("\nNext steps:");
    println!("  cd {}", project_name);
  } else {
    println!("✅ Project created successfully!");
  }
  Ok(())
}
