use anyhow::{anyhow, Result};
use build_fs_tree::{Build, MergeableFileSystemTree};
use clap::Parser;
use colored::Colorize;
use scaffold_tnesh_zome::scaffold_tnesh_zome;
use std::{
    ffi::OsString,
    fs,
    path::PathBuf,
    process::{Command, ExitCode},
};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The name of the zome to be scaffolded
    #[clap(long)]
    pub zome_name: Option<String>,

    /// The name of the NPM org with which to scaffold the zome
    #[clap(long)]
    pub npm_organization: Option<String>,

    /// The name of the github org with which to scaffold the zome
    #[clap(long)]
    pub github_organization: Option<String>,

    /// The name of the cachix cache with which to scaffold the zome
    #[clap(long)]
    pub cachix_cache: Option<String>,

    /// The path of the file tree to modify.
    #[clap(long, default_value = "./.")]
    pub path: PathBuf,
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

    let (name, file_tree) = scaffold_tnesh_zome(
        args.zome_name,
        args.npm_organization,
        args.github_organization,
        args.cachix_cache,
    )?;

    let folder_name = format!("{name}-zome");

    let runtime_path = args.path.join(&folder_name);

    if let Ok(path) = std::fs::canonicalize(&runtime_path) {
        if path.exists() {
            return Err(anyhow!(
                "The directory {name}-zome already exists: choose another name"
            ));
        }
    }

    fs::create_dir_all(&runtime_path)?;

    let file_tree = MergeableFileSystemTree::<OsString, String>::from(file_tree);

    file_tree.build(&runtime_path)?;

    println!(
        "{}",
        format!("Successfully scaffolded the {name} TNESH zome").green()
    );

    Ok(())
}
