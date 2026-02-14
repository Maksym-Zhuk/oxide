use crate::{
    cli::{Cli, commands::Commands},
    prompts::variables::{
        ask_build_tool, ask_framework, ask_language, ask_platform, ask_project_name,
    },
    templates::generator::extract_template,
    utils::{fs::generate_path, validate::validate_project_name},
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
        Commands::Create { name } => {
            let project_name = match name {
                Some(n) => n,
                None => ask_project_name()?,
            };

            validate_project_name(&project_name)?;

            let framework = ask_framework()?;

            let build_tool = framework
                .needs_build_tool()
                .then(|| ask_build_tool(framework))
                .transpose()?;

            let language = if framework.needs_choose_language() {
                Some(ask_language()?)
            } else {
                Some(prompts::Language::TypeScript)
            };

            let platform = framework
                .needs_choose_paltform()
                .then(|| ask_platform(framework, &build_tool))
                .transpose()?;

            let path = generate_path(&language, &build_tool, &framework, &platform);

            let template = TEMPLATES
                .get_dir(&path)
                .ok_or_else(|| format!("Template not found: {}", path))?;

            extract_template(template, &project_name)?;

            println!("✅ Project created successfully!");
            println!("\nNext steps:");
            println!("  cd {}", project_name);
            println!("  npm install");
            println!("  npm run dev");
        }
    };

    Ok(())
}
