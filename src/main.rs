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
    /// Run the checks; fail on any failing check or new finding.
    Check {
        /// Directory holding rules.toml, checks.toml and baseline.json.
        #[arg(long, default_value = ".charter")]
        dir: PathBuf,
        /// Run checks tied to this event as well as the unconditional ones.
        #[arg(long)]
        on: Option<String>,
        /// Print machine-readable JSON.
        #[arg(long)]
        json: bool,
        /// Limit SARIF findings to files changed since this revision.
        #[arg(long)]
        changed: Option<String>,
    },
    /// Accept the current findings of every sarif check as the baseline.
    Baseline {
        /// Directory holding rules.toml, checks.toml and baseline.json.
        #[arg(long, default_value = ".charter")]
        dir: PathBuf,
    },
    /// Print the reviewer rubric.
    Rubric {
        #[arg(long, default_value = ".charter")]
        dir: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Verify a review table against the policy.
    VerifyReview {
        file: PathBuf,
        #[arg(long, default_value = ".charter")]
        dir: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Verify required artifact sections in a pull request body.
    VerifyPr {
        file: PathBuf,
        #[arg(long, default_value = ".charter")]
        dir: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Cmd::Doctor { dir, json } => edge::doctor(&dir, json),
        Cmd::Check {
            dir,
            on,
            json,
            changed,
        } => edge::check(&dir, on.as_deref(), json, changed.as_deref()),
        Cmd::Baseline { dir } => edge::baseline(&dir),
        Cmd::Rubric { dir, json } => edge::rubric(&dir, json),
        Cmd::VerifyReview { file, dir, json } => edge::verify_review(&file, &dir, json),
        Cmd::VerifyPr { file, dir, json } => edge::verify_pr(&file, &dir, json),
    }
}
