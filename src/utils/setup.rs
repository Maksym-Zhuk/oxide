use std::process::Command;

use anyhow::{Result, anyhow};

use crate::{
  paths::OxidePaths,
  prompts::{
    self, BuildTool, FrameworkConfig, Language, PackageManager, parse_platform,
    variables::{ask_build_tool, ask_language, ask_package_manager, ask_platform, ask_user_name},
  },
  templates::{generator::extract_template, install::install_template, loader::get_files},
  utils::fs::generate_path,
};

pub struct SetupProjectOptions<F> {
  pub project_name: Option<String>,
  pub framework: F,
  pub build_tool: Option<BuildTool>,
  pub language: Option<Language>,
  pub platform: Option<String>,
  pub package_manager: Option<PackageManager>,
}

pub async fn setup_project<F>(
  setup_options: SetupProjectOptions<F>,
  home_dir: &OxidePaths,
  is_install: bool,
) -> Result<()>
where
  F: FrameworkConfig + std::fmt::Display + std::fmt::Debug,
{
  let build_tool =
    if setup_options.build_tool.is_none() && setup_options.framework.needs_build_tool() {
      Some(ask_build_tool(&setup_options.framework)?)
    } else {
      setup_options.build_tool
    };

  let language = match setup_options.language {
    Some(l) => Some(l),
    None if setup_options.framework.needs_choose_language() => Some(ask_language()?),
    None => Some(prompts::Language::TypeScript),
  };

  let platform = match setup_options.platform {
    Some(p) => Some(parse_platform(
      &p,
      &setup_options.framework.to_string(),
      &build_tool,
    )?),
    None if setup_options.framework.needs_choose_paltform(&build_tool) => {
      Some(ask_platform(&setup_options.framework, &build_tool)?)
    }
    None => None,
  };

  let path = generate_path(
    &language,
    &build_tool,
    &setup_options.framework.to_string().replace(" ", ""),
    &platform,
  );

  let template_path = home_dir.home.join("cache").join("templates");

  if is_install {
    install_template(&template_path, &path).await?;
  } else {
    let project_name = setup_options
      .project_name
      .ok_or_else(|| anyhow!("Project name is required"))?;

    let files = get_files(path, &template_path).await?;

    let tauri_user_name = if setup_options.framework.is_tauri() {
      Some(ask_user_name()?)
    } else {
      None
    };

    let package_manager = match setup_options.package_manager {
      Some(pm) => pm,
      None => ask_package_manager()?,
    };

    extract_template(&files, &project_name, tauri_user_name)?;

    let status: std::process::ExitStatus = Command::new(package_manager.to_string())
      .arg("install")
      .current_dir(&project_name)
      .status()
      .map_err(|e| anyhow!(e))?;

    if !status.success() {
      return Err(anyhow!(
        "{} install failed with code {:?}",
        package_manager,
        status.code()
      ));
    }

    println!("✅ Project created successfully!");
    println!("\nNext steps:");
    println!("  cd {}", project_name);
    println!("  {} run dev", package_manager);
  }

  Ok(())
}
