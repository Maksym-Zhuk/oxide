use crate::{
  cache::{get_installed_templates, remove_template_from_cache},
  cli::{Cli, commands::Commands},
  paths::OxidePaths,
  prompts::{
    BackendTool, BuildTool, DesktopRuntime, FrontendTool, Language, MetaFramework, MobileTool,
    PackageManager, ProjectLayer,
    variables::{
      ask_backend_framework, ask_desctop_framework, ask_frontend_framework, ask_meta_framework,
      ask_mobile_framework, ask_project_layer, ask_project_name,
    },
  },
  templates::install::install_template_by_name,
  utils::{
    setup::{SetupProjectOptions, setup_project},
    validate::validate_project_name,
  },
};
use anyhow::Result;
use clap::Parser;

pub mod cache;
pub mod cli;
pub mod config;
pub mod paths;
pub mod prompts;
pub mod templates;
pub mod utils;

pub struct ProjectInitOptions {
  pub name: Option<String>,
  pub layer: Option<ProjectLayer>,
  pub framework: Option<String>,
  pub build_tool: Option<BuildTool>,
  pub language: Option<Language>,
  pub platform: Option<String>,
  pub package_manager: Option<PackageManager>,
  pub template_name: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
  let oxide_paths = OxidePaths::new()?;

  oxide_paths.ensure_directories()?;

  let cli = Cli::parse();
  let template_path = oxide_paths.home.join("cache").join("templates");

  match cli.command {
    Commands::New {
      name,
      layer,
      framework,
      build_tool,
      language,
      platform,
      package_manager,
    } => {
      let project_name = match name {
        Some(n) => n,
        None => ask_project_name()?,
      };

      validate_project_name(&project_name)?;

      run_project_flow(
        ProjectInitOptions {
          name: Some(project_name),
          layer,
          framework,
          build_tool,
          language,
          platform,
          package_manager,
          template_name: None,
        },
        &oxide_paths,
        false,
      )
      .await?
    }
    Commands::Install {
      template_name,
      layer,
      framework,
      build_tool,
      language,
      platform,
    } => {
      run_project_flow(
        ProjectInitOptions {
          name: None,
          layer,
          framework,
          build_tool,
          language,
          platform,
          package_manager: None,
          template_name,
        },
        &oxide_paths,
        true,
      )
      .await?
    }
    Commands::Delete { template_name } => {
      remove_template_from_cache(&template_path, &template_name)?;
    }
    Commands::Installed {} => get_installed_templates(&template_path)?,
  }

  Ok(())
}

pub async fn run_project_flow(
  options: ProjectInitOptions,
  oxide_paths: &OxidePaths,
  is_install: bool,
) -> Result<()> {
  if let Some(tn) = options.template_name {
    install_template_by_name(&oxide_paths.home.join("cache").join("templates"), tn).await?;
  } else {
    let project_layer = match options.layer {
      Some(l) => l,
      None => ask_project_layer()?,
    };

    match project_layer {
      ProjectLayer::Frontend => {
        let framework = match options.framework {
          Some(f) => f.parse::<FrontendTool>()?,
          None => ask_frontend_framework()?,
        };
        setup_project::<FrontendTool>(
          SetupProjectOptions {
            project_name: options.name,
            framework,
            build_tool: options.build_tool,
            language: options.language,
            platform: options.platform,
            package_manager: options.package_manager,
          },
          oxide_paths,
          is_install,
        )
        .await?
      }
      ProjectLayer::Meta => {
        let framework = match options.framework {
          Some(f) => f.parse::<MetaFramework>()?,
          None => ask_meta_framework()?,
        };
        setup_project::<MetaFramework>(
          SetupProjectOptions {
            project_name: options.name,
            framework,
            build_tool: options.build_tool,
            language: options.language,
            platform: options.platform,
            package_manager: options.package_manager,
          },
          oxide_paths,
          is_install,
        )
        .await?;
      }
      ProjectLayer::Backend => {
        let framework = match options.framework {
          Some(f) => f.parse::<BackendTool>()?,
          None => ask_backend_framework()?,
        };
        setup_project::<BackendTool>(
          SetupProjectOptions {
            project_name: options.name,
            framework,
            build_tool: options.build_tool,
            language: options.language,
            platform: options.platform,
            package_manager: options.package_manager,
          },
          oxide_paths,
          is_install,
        )
        .await?;
      }
      ProjectLayer::Desktop => {
        let framework = match options.framework {
          Some(f) => f.parse::<DesktopRuntime>()?,
          None => ask_desctop_framework()?,
        };
        setup_project::<DesktopRuntime>(
          SetupProjectOptions {
            project_name: options.name,
            framework,
            build_tool: options.build_tool,
            language: options.language,
            platform: options.platform,
            package_manager: options.package_manager,
          },
          oxide_paths,
          is_install,
        )
        .await?;
      }
      ProjectLayer::Mobile => {
        let framework = match options.framework {
          Some(f) => f.parse::<MobileTool>()?,
          None => ask_mobile_framework()?,
        };
        setup_project::<MobileTool>(
          SetupProjectOptions {
            project_name: options.name,
            framework,
            build_tool: options.build_tool,
            language: options.language,
            platform: options.platform,
            package_manager: options.package_manager,
          },
          oxide_paths,
          is_install,
        )
        .await?;
      }
    };
  }
  Ok(())
}
