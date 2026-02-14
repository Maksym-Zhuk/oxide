use clap::Parser;

pub mod commands;
use commands::Commands;

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}
