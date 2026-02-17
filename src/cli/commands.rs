use clap::Subcommand;

use crate::prompts::{BuildTool, Language, PackageManager, ProjectLayer};

#[derive(Subcommand)]
pub enum Commands {
  Create {
    name: Option<String>,

    #[arg(short, long)]
    layer: Option<ProjectLayer>,

    #[arg(short, long)]
    framework: Option<String>,

    #[arg(short, long)]
    build_tool: Option<BuildTool>,

    #[arg(short = 'L', long, alias = "lang")]
    language: Option<Language>,

    #[arg(short, long)]
    platform: Option<String>,

    #[arg(short = 'm', long)]
    package_manager: Option<PackageManager>,
  },
}
