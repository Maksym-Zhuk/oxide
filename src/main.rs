use crate::{
  cli::{Cli, commands::Commands},
  prompts::{
    BackendTool, DesktopRuntime, FrontendTool, MetaFramework, MobileTool, ProjectLayer,
    variables::{
      ask_backend_framework, ask_desctop_framework, ask_frontend_framework, ask_meta_framework,
      ask_mobile_framework, ask_project_layer, ask_project_name,
    },
  },
  utils::{setup::setup_project, validate::validate_project_name},
};
use clap::Parser;
use include_dir::{Dir, include_dir};

pub mod cli;
pub mod config;
pub mod prompts;
pub mod templates;
pub mod utils;

static TEMPLATES: Dir = include_dir!("$CARGO_MANIFEST_DIR/templates");

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let cli = Cli::parse();

  match cli.command {
    Commands::Create {
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

      let project_layer = match layer {
        Some(l) => l,
        None => ask_project_layer()?,
      };

      match project_layer {
        ProjectLayer::Frontend => {
          let framework = match framework {
            Some(f) => f.parse::<FrontendTool>()?,
            None => ask_frontend_framework()?,
          };
          setup_project(
            &project_name,
            framework,
            build_tool,
            language,
            platform,
            package_manager,
          )
          .await?;
        }
        ProjectLayer::Meta => {
          let framework = match framework {
            Some(f) => f.parse::<MetaFramework>()?,
            None => ask_meta_framework()?,
          };
          setup_project(
            &project_name,
            framework,
            build_tool,
            language,
            platform,
            package_manager,
          )
          .await?;
        }
        ProjectLayer::Backend => {
          let framework = match framework {
            Some(f) => f.parse::<BackendTool>()?,
            None => ask_backend_framework()?,
          };
          setup_project(
            &project_name,
            framework,
            build_tool,
            language,
            platform,
            package_manager,
          )
          .await?;
        }
        ProjectLayer::Desktop => {
          let framework = match framework {
            Some(f) => f.parse::<DesktopRuntime>()?,
            None => ask_desctop_framework()?,
          };
          setup_project(
            &project_name,
            framework,
            build_tool,
            language,
            platform,
            package_manager,
          )
          .await?;
        }
        ProjectLayer::Mobile => {
          let framework = match framework {
            Some(f) => f.parse::<MobileTool>()?,
            None => ask_mobile_framework()?,
          };
          setup_project(
            &project_name,
            framework,
            build_tool,
            language,
            platform,
            package_manager,
          )
          .await?;
        }
      };
    }
  }

  Ok(())
}
