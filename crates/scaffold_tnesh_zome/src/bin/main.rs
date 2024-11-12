use anyhow::{anyhow, Result};
use build_fs_tree::{Build, MergeableFileSystemTree};
use clap::Parser;
use colored::Colorize;
use git2::{IndexAddOption, Repository, RepositoryInitOptions};
use scaffold_tnesh_zome::scaffold_tnesh_zome;
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
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

    let project_path = args.path.join(&folder_name);

    if let Ok(path) = std::fs::canonicalize(&project_path) {
        if path.exists() {
            return Err(anyhow!(
                "The directory {name}-zome already exists: choose another name"
            ));
        }
    }

    fs::create_dir_all(&project_path)?;

    let file_tree = MergeableFileSystemTree::<OsString, String>::from(file_tree);

    file_tree.build(&project_path)?;

    setup_git_environment(&project_path)?;

    println!(
        "{}",
        format!("Successfully scaffolded the {name} TNESH zome").green()
    );

    Ok(())
}

fn setup_git_environment<P: AsRef<Path>>(path: P) -> anyhow::Result<()> {
    if let Err(e) = (|| {
        let repo = Repository::init_opts(path, RepositoryInitOptions::new().initial_head("main"))?;
        let mut index = repo.index()?;
        index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
        index.write()?;
        Ok::<_, git2::Error>(())
    })() {
        println!(
            "{}{}",
            "Warning: Failed to set up git repository: ".yellow(),
            e.to_string().yellow()
        );
    }

    Ok(())
}
