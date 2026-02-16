use std::process::Command;

use crate::{
  TEMPLATES,
  prompts::{
    self, FrameworkConfig,
    variables::{ask_build_tool, ask_language, ask_package_manager, ask_platform, ask_user_name},
  },
  templates::generator::extract_template,
  utils::fs::generate_path,
};

pub async fn setup_project<F>(
  framework: F,
  project_name: &str,
) -> Result<(), Box<dyn std::error::Error>>
where
  F: FrameworkConfig + std::fmt::Display + std::fmt::Debug,
{
  let build_tool = if framework.needs_build_tool() {
    Some(ask_build_tool(&framework)?)
  } else {
    None
  };

  let language = if framework.needs_choose_language() {
    Some(ask_language()?)
  } else {
    Some(prompts::Language::TypeScript)
  };

  let platform = if framework.needs_choose_paltform(&build_tool) {
    Some(ask_platform(&framework, &build_tool)?)
  } else {
    None
  };

  let path = generate_path(
    &language,
    &build_tool,
    &framework.to_string().replace(" ", ""),
    &platform,
  );

  let template = TEMPLATES
    .get_dir(&path)
    .ok_or_else(|| format!("Template not found: {}", path))?;

  let tauri_user_name = if framework.is_tauri() {
    Some(ask_user_name()?)
  } else {
    None
  };

  let package_manager = ask_package_manager()?;

  extract_template(template, project_name, tauri_user_name)?;

  let status: std::process::ExitStatus = Command::new(package_manager.to_string())
    .arg("install")
    .current_dir(project_name)
    .status()
    .map_err(|e: std::io::Error| -> Box<dyn std::error::Error> { e.into() })?;

  if !status.success() {
    return Err(format!("bun install failed with code {:?}", status.code()).into());
  }

  println!("✅ Project created successfully!");
  println!("\nNext steps:");
  println!("  cd {}", project_name);
  println!("  {} run dev", package_manager);

  Ok(())
}
