use crate::prompts::{
  BackendTool, BuildTool, DesktopRuntime, FrameworkConfig, FrontendTool, Language, MetaFramework,
  MobileTool, PackageManager, Platform, ProjectLayer,
};
use anyhow::{Result, anyhow};
use inquire::{Select, Text};
use regex::Regex;

pub fn ask_project_name() -> Result<String> {
  Ok(
    Text::new("Project name:")
      .with_placeholder("my-awesome-app")
      .with_help_message("Enter your project name")
      .prompt()?,
  )
}

pub fn ask_project_layer() -> Result<ProjectLayer> {
  let project_layers = vec![
    ProjectLayer::Frontend,
    ProjectLayer::Backend,
    ProjectLayer::Meta,
    ProjectLayer::Desktop,
    ProjectLayer::Mobile,
  ];

  let project_layer = Select::new("Select a layer:", project_layers).prompt()?;
  Ok(project_layer)
}

pub fn ask_frontend_framework() -> Result<FrontendTool> {
  let tools = vec![
    FrontendTool::React,
    FrontendTool::Preact,
    FrontendTool::Vue,
    FrontendTool::Svelte,
    FrontendTool::Solid,
    FrontendTool::Lit,
    FrontendTool::Qwik,
    FrontendTool::Angular,
  ];

  let tool = Select::new("Select a tool:", tools).prompt()?;
  Ok(tool)
}

pub fn ask_meta_framework() -> Result<MetaFramework> {
  let frameworks = vec![MetaFramework::Next, MetaFramework::Nuxt];

  let framework = Select::new("Select a framework:", frameworks).prompt()?;
  Ok(framework)
}

pub fn ask_backend_framework() -> Result<BackendTool> {
  let tools = vec![BackendTool::Nest];

  let tool = Select::new("Select a tool:", tools).prompt()?;
  Ok(tool)
}

pub fn ask_desctop_framework() -> Result<DesktopRuntime> {
  let frameworks = vec![DesktopRuntime::Tauri, DesktopRuntime::Electron];

  let framework = Select::new("Select a framework:", frameworks).prompt()?;
  Ok(framework)
}

pub fn ask_mobile_framework() -> Result<MobileTool> {
  let tools = vec![MobileTool::ReactNative];

  let tool = Select::new("Select a tool:", tools).prompt()?;
  println!("{}", tool);
  Ok(tool)
}

pub fn ask_build_tool<F>(framework: &F) -> Result<BuildTool>
where
  F: FrameworkConfig,
{
  let build_tool = framework.compatible_build_tools();

  let build_tool = Select::new("Select a build tool:", build_tool).prompt()?;
  Ok(build_tool)
}

pub fn ask_language() -> Result<Language> {
  let languages = vec![Language::TypeScript, Language::JavaScript];

  let language = Select::new("Select a language:", languages).prompt()?;
  Ok(language)
}

pub fn ask_platform<F>(framework: &F, build_tool: &Option<BuildTool>) -> Result<Platform>
where
  F: FrameworkConfig,
{
  let platforms = framework.compatible_platforms(build_tool);

  if platforms.is_empty() {
    return Err(anyhow!(
      "No available platforms for the selected framework/build tool"
    ));
  }

  let platform = Select::new("Select a platform:", platforms).prompt()?;
  Ok(platform)
}

pub fn ask_user_name() -> Result<String> {
  let valid_chars = Regex::new(r"^[a-zA-Z0-9_\-\.]+$").unwrap();

  let user_name = Text::new("Your name:")
    .with_placeholder("maksym")
    .with_help_message("Enter your name")
    .prompt()?
    .to_lowercase();

  if !valid_chars.is_match(&user_name) {
    return Err(anyhow!(
      "Name can only contain letters, numbers, hyphens, underscores, and dots"
    ));
  }

  Ok(user_name)
}

pub fn ask_package_manager() -> Result<PackageManager> {
  let package_managers = vec![
    PackageManager::NPM,
    PackageManager::Yarn,
    PackageManager::PNPM,
    PackageManager::Bun,
  ];

  let package_manager = Select::new("Select a package manager:", package_managers).prompt()?;
  Ok(package_manager)
}
