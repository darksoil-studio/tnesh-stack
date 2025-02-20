use anyhow::{anyhow, Result};
use clap::Parser;
use colored::Colorize;
use npm_scaffolding_utils::PackageManager;
use std::process::ExitCode;

#[derive(Parser, Debug)]
pub struct Args {
    /// The package manager that is used in this repository
    #[clap(long)]
    pub package_manager: Option<String>,
}

fn main() -> ExitCode {
    if let Err(err) = internal_main() {
        eprintln!("{}", format!("Error: {err:?}").red());
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn internal_main() -> Result<()> {
    let args = Args::parse();

    let package_manager = match args.package_manager {
        Some(p) => Some(PackageManager::try_from(p).map_err(|err| anyhow!(err))?),
        None => None,
    };

    sync_npm_rev_dependencies_with_nix::synchronize_npm_rev_dependencies_with_nix(package_manager)?;

    Ok(())
}
