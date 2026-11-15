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
        /// Explain configured mappings without executing checks.
        #[arg(long, conflicts_with = "json")]
        explain: bool,
    },
    /// Run the checks; fail on any failing check or new finding.
    Check(CheckArgs),
    /// Accept the current findings of every sarif check as the baseline.
    Baseline {
        /// Accept findings from exactly one SARIF check.
        #[arg(long)]
        check: Option<String>,
        /// Print proposed JSON without writing baseline.json; checks still run.
        #[arg(long)]
        preview: bool,
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
    /// Run SARIF checks for the file named in hook input JSON.
    Hook {
        #[arg(long, default_value = ".charter")]
        dir: PathBuf,
    },
    /// Write starter rules and checks files.
    Init {
        /// Import a complete checks file, validated before writing.
        #[arg(long)]
        checks_from: Option<PathBuf>,
        #[arg(long, default_value = ".charter")]
        dir: PathBuf,
    },
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Cmd::Doctor { dir, json, explain } => edge::doctor_with_explanation(&dir, json, explain),
        Cmd::Check(args) => run_check(&args),
        Cmd::Baseline {
            dir,
            check,
            preview,
        } => edge::baseline_with_options(&dir, check.as_deref(), preview),
        Cmd::Rubric { dir, json } => edge::rubric(&dir, json),
        Cmd::VerifyReview { file, dir, json } => edge::verify_review(&file, &dir, json),
        Cmd::VerifyPr { file, dir, json } => edge::verify_pr(&file, &dir, json),
        Cmd::Hook { dir } => edge::hook(&dir),
        Cmd::Init { dir, checks_from } => edge::init_with_checks(&dir, checks_from.as_deref()),
    }
}

#[derive(clap::Args)]
struct CheckArgs {
    /// Directory holding rules.toml, checks.toml and baseline.json.
    #[arg(long, default_value = ".charter")]
    dir: PathBuf,
    /// Run checks tied to this event as well as the unconditional ones.
    #[arg(long)]
    on: Option<String>,
    /// Print machine-readable JSON.
    #[arg(long)]
    json: bool,
    /// Report completed execution observations; does not certify rule enforcement.
    #[arg(long, conflicts_with = "json")]
    report: bool,
    /// Limit SARIF findings to files changed since this revision.
    #[arg(long)]
    changed: Option<String>,
    /// Filter SARIF findings to staged, unstaged and untracked paths; checks read current files.
    #[arg(long, conflicts_with = "changed")]
    working_tree: bool,
}

fn run_check(args: &CheckArgs) -> ExitCode {
    let on = args.on.as_deref();
    let changed = args.changed.as_deref();
    if args.report {
        edge::check_with_report(&args.dir, on, changed, args.working_tree)
    } else if args.working_tree {
        edge::check_working_tree(&args.dir, on, args.json)
    } else {
        edge::check(&args.dir, on, args.json, changed)
    }
}
