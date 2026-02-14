use crate::prompts::{BuildTool, Framework, Language, Platform};
use inquire::{Select, Text};
use regex::Regex;

pub fn ask_project_name() -> Result<String, Box<dyn std::error::Error>> {
  Ok(
    Text::new("Project name:")
      .with_placeholder("my-awesome-app")
      .with_help_message("Enter your project name")
      .prompt()?,
  )
}

pub fn ask_framework() -> Result<Framework, Box<dyn std::error::Error>> {
  let frameworks = vec![
    Framework::React,
    Framework::Preact,
    Framework::Vue,
    Framework::Svelte,
    Framework::Solid,
    Framework::Lit,
    Framework::Qwik,
    Framework::Angular,
    Framework::Nest,
    Framework::Next,
    Framework::Nuxt,
    Framework::Electron,
    Framework::Tauri,
  ];

  let framework = Select::new("Select a framework:", frameworks).prompt()?;
  Ok(framework)
}

pub fn ask_build_tool(framework: Framework) -> Result<BuildTool, Box<dyn std::error::Error>> {
  let build_tool = framework.compatible_build_tools();

  let build_tool = Select::new("Select a build tool:", build_tool).prompt()?;
  Ok(build_tool)
}

pub fn ask_language() -> Result<Language, Box<dyn std::error::Error>> {
  let languages = vec![Language::TypeScript, Language::JavaScript];

  let language = Select::new("Select a language:", languages).prompt()?;
  Ok(language)
}

pub fn ask_platform(
  framework: Framework,
  build_tool: &Option<BuildTool>,
) -> Result<Platform, Box<dyn std::error::Error>> {
  let platforms = framework.compatible_platform(build_tool);

  if platforms.is_empty() {
    return Err("No available platforms for the selected framework/build tool".into());
  }

  let platform = Select::new("Select a platform:", platforms).prompt()?;
  Ok(platform)
}

pub fn ask_user_name() -> Result<String, Box<dyn std::error::Error>> {
  let valid_chars = Regex::new(r"^[a-zA-Z]+$").unwrap();

  let user_name = Text::new("Your name:")
    .with_placeholder("maksym")
    .with_help_message("Enter your name")
    .prompt()?
    .to_lowercase();

  if !valid_chars.is_match(&user_name) {
    return Err(
      "Project name can only contain letters, numbers, hyphens, underscores, and dots"
        .to_string()
        .into(),
    );
  }

  Ok(user_name)
}
