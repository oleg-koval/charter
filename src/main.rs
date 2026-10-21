use std::path::PathBuf;
use std::process::ExitCode;

use charter_cli::edge;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "charter", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Check that every rule has every promised layer mapped.
    Doctor {
        /// Directory holding rules.toml and checks.toml.
        #[arg(long, default_value = ".charter")]
        dir: PathBuf,
        /// Print machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Cmd::Doctor { dir, json } => edge::doctor(&dir, json),
    }
}
