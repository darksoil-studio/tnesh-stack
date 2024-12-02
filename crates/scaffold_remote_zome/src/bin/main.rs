use anyhow::Result;
use build_fs_tree::{Build, MergeableFileSystemTree};
use clap::Parser;
use colored::Colorize;
use dialoguer::Confirm;
use scaffold_remote_zome::{scaffold_remote_zome, ScaffoldRemoteZomeError};
use std::{
    ffi::OsString,
    path::PathBuf,
    process::{Command, ExitCode},
};
use sync_npm_git_dependencies_with_nix::synchronize_npm_git_dependencies_with_nix;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the module zome that's being scaffolded
    module_name: String,

    /// Run the command in CI, skipping making questions to the users
    #[arg(long)]
    ci: bool,

    /// Name of the integrity zome that's being scaffolded
    #[arg(long)]
    integrity_zome_name: Option<String>,

    /// Name of the coordinator zome that's being scaffolded
    #[arg(long)]
    coordinator_zome_name: Option<String>,

    /// URL for the git repository of the zome that's being scaffolded
    #[arg(long)]
    remote_zome_git_url: String,

    /// Branch for the git repository of the zome that's being scaffolded
    #[arg(long)]
    remote_zome_git_branch: Option<String>,

    /// Name of the UI package for the zome that's being scaffolded
    #[arg(long)]
    remote_npm_package_name: String,

    /// Internal path of the UI package
    #[arg(long)]
    remote_npm_package_path: PathBuf,

    /// DNA of the local repository in which the zome should be scaffolded
    #[arg(long)]
    local_dna_to_add_the_zome_to: Option<String>,

    /// NPM package for the local repository in which the UI for the zome should be scaffolded
    #[arg(long)]
    local_npm_package_to_add_the_ui_to: Option<String>,

    /// <*-context> element that will be added to the top level context setting for the app's UI
    #[arg(long)]
    context_element: Option<String>,

    /// JS import location for the context element that will be added to the top level context setting for the app's UI
    #[arg(long)]
    context_element_import: Option<String>,

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

    let zomes_prompt = match (
        args.integrity_zome_name.clone(),
        args.coordinator_zome_name.clone(),
    ) {
        (Some(integrity), Some(coordinator)) => format!(
            r#"Add the "{integrity}" integrity zome and the "{coordinator}" coordinator zome to the dna.nix that you select."#
        ),
        (Some(integrity), None) => {
            format!(r#"Add the "{integrity}" integrity zome to the dna.nix that you select."#)
        }
        (None, Some(coordinator)) => {
            format!(r#"Add the "{coordinator}" coordinator zome to the dna.nix that you select."#)
        }
        (None, None) => {
            return Err(ScaffoldRemoteZomeError::NoZomesSpecifiedError)?;
        }
    };

    let context_prompt = match &args.context_element {
        Some(ce) => format!(
            r#"- Set up the "<{ce}>" element in your top level app component so that you can directly import the UI elements you need from the {} package.
"#,
            args.remote_npm_package_name
        ),
        None => format!(""),
    };

    let confirm = match args.ci {
        true => true,
        false => {
            println!(
                r#"
You are about to add the {} zome to your hApp.

These are the steps that will be taken:

- Add the flake input "{}" to your flake.nix.
- {}
- Add the UI package "{}" as a dependency of your UI package.
{}
"#,
                args.module_name,
                args.remote_zome_git_url,
                zomes_prompt,
                args.remote_npm_package_name,
                context_prompt,
            );

            Confirm::new()
                .with_prompt("Are you ready to continue?")
                .interact()?
        }
    };

    if !confirm {
        return Ok(());
    }

    let file_tree = file_tree_utils::load_directory_into_memory(&args.path)?;

    let file_tree = scaffold_remote_zome(
        file_tree,
        args.module_name.clone(),
        args.integrity_zome_name.clone(),
        args.coordinator_zome_name.clone(),
        args.remote_zome_git_url,
        args.remote_zome_git_branch,
        args.remote_npm_package_name,
        args.remote_npm_package_path,
        args.local_dna_to_add_the_zome_to,
        args.local_npm_package_to_add_the_ui_to,
        args.context_element,
        args.context_element_import,
    )?;

    let file_tree = MergeableFileSystemTree::<OsString, String>::from(file_tree);

    file_tree.build(&args.path)?;

    // Run nix flake update

    // Run nix develop -c bash "pnpm install"

    println!(
        "{}",
        format!("Successfully scaffolded zome {}", args.module_name.bold()).green()
    );

    println!("Running nix flake update...");
    println!("");
    Command::new("nix").args(["flake", "update"]).output()?;

    synchronize_npm_git_dependencies_with_nix()?;

    Ok(())
}
