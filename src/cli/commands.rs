use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
  Create { name: Option<String> },
}
