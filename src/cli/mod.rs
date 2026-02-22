use clap::Parser;

pub mod commands;
use commands::Commands;

#[derive(Parser)]
#[command(version)]
pub struct Cli {
  #[command(subcommand)]
  pub command: Commands,
}
