use anyhow::Result;
use colored::Colorize;
use ignore::Walk;
use npm_scaffolding_utils::guess_or_choose_package_manager;
use parse_flake_lock::{FlakeLock, FlakeLockParseError, Node};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    env::current_dir,
    fs::File,
    io::BufReader,
    process::{Command, Stdio},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SynchronizeNpmGitDependenciesWithNixError {
    #[error(transparent)]
    FlakeLockParseError(#[from] FlakeLockParseError),

    #[error("IO error: {0}")]
    StdIoError(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    SerdeJsonError(#[from] serde_json::Error),

    #[error(transparent)]
    RegexError(#[from] regex::Error),

    #[error(transparent)]
    FileTreeError(#[from] file_tree_utils::FileTreeError),

    #[error(transparent)]
    NpmScaffoldingUtilsError(#[from] npm_scaffolding_utils::NpmScaffoldingUtilsError),

    #[error("Error parsing git {0} for dependency {1}")]
    ParseGitRepositoryError(String, String),

    #[error("Error parsing the rev. dependency {0}")]
    RevDependencyError(String),

    #[error("Error getting the NPM repo for dependency {0}: {1}")]
    NpmRepoError(String, String),

    #[error("Error getting the NPM version for dependency {0}: {1}")]
    NpmShowError(String, String),
}

pub fn synchronize_npm_rev_dependencies_with_nix(
) -> Result<(), SynchronizeNpmGitDependenciesWithNixError> {
    let current_dir = current_dir()?;
    let flake_lock = current_dir.join("flake.lock");

    // Return silently if no "flake.lock" file exists
    if !flake_lock.exists() {
        return Ok(());
    }

    let flake_lock = FlakeLock::new(flake_lock.as_path())?;

    let mut announced = false;
    let mut replaced_some_dep = false;

    for entry in Walk::new(".").into_iter().filter_map(|e| e.ok()) {
        let f_name = entry.file_name().to_string_lossy();

        if f_name == "package.json" {
            let mut replaced_some_dep_in_this_file = false;
            let file = File::open(entry.path())?;
            let reader = BufReader::new(file);
            let mut package_json_contents: Value = serde_json::from_reader(reader)?;

            if let Some(Value::Object(deps)) = package_json_contents.get_mut("dependencies") {
                let re = Regex::new(r#"^(.*)-rev\.(.*)$"#)?;

                for (package, dependency_source) in deps {
                    if let Value::String(dependency_source_str) = dependency_source.clone() {
                        if let Some(captures) = re.captures(&dependency_source_str) {
                            let git_repo = get_repo(package)?;
                            let _version = captures
                                .get(1)
                                .ok_or(
                                    SynchronizeNpmGitDependenciesWithNixError::RevDependencyError(
                                        dependency_source_str.clone(),
                                    ),
                                )?
                                .as_str();
                            let revision = captures
                                .get(2)
                                .ok_or(
                                    SynchronizeNpmGitDependenciesWithNixError::RevDependencyError(
                                        dependency_source_str.clone(),
                                    ),
                                )?
                                .as_str();

                            for root_node in flake_lock.root.values() {
                                if let Node::Repo(repo_node) = root_node {
                                    if repo_node.locked.node_type == git_repo.git_type
                                        && repo_node.locked.owner == git_repo.owner
                                        && repo_node.locked.repo == git_repo.repo
                                        && revision != repo_node.locked.rev
                                    {
                                        let new_version =
                                            get_version(package, &repo_node.locked.rev)?;
                                        *dependency_source =
                                            Value::String(format!("{new_version}"));

                                        if !announced {
                                            announced = true;
                                            println!("");
                                            println!(
                                                "Synchronizing npm git dependencies with the upstream nix sources..."
                                            );
                                        }

                                        println!(
                                            "  - Setting dependency \"{package}\" in file {:?} to rev \"{}\"",
                                            entry.path(), new_version
                                        );
                                        replaced_some_dep = true;
                                        replaced_some_dep_in_this_file = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if replaced_some_dep_in_this_file {
                let st = serde_json::to_string_pretty(&package_json_contents)?;

                std::fs::write(entry.path(), st)?;
            }
        }
    }

    let file_tree = file_tree_utils::load_directory_into_memory(&current_dir)?;
    let package_manager = guess_or_choose_package_manager(&file_tree)?
        .to_string()
        .to_lowercase();

    if replaced_some_dep {
        println!("Running {package_manager} install...");
        println!("");
        Command::new(package_manager)
            .arg("install")
            .stdout(Stdio::inherit())
            .output()?;
        println!(
            "{}",
            "Successfully synchronized npm dependencies with nix".green()
        );
    }

    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
struct GitRepo {
    owner: String,
    repo: String,
    git_type: String,
}

fn get_repo(npm_package: &String) -> Result<GitRepo, SynchronizeNpmGitDependenciesWithNixError> {
    let output = Command::new("npm")
        .args(["repo", npm_package, "--no-browser"])
        .stderr(Stdio::null())
        .output()?;
    let npm_repo_output = String::from_utf8(output.stdout).map_err(|err| {
        SynchronizeNpmGitDependenciesWithNixError::NpmRepoError(
            npm_package.clone(),
            format!("{err:?}"),
        )
    })?;

    let re = Regex::new(r#"https://(github|gitlab).com/([^/]+)/([^\n]+)"#)?;
    let Some(captures) = re.captures(&npm_repo_output) else {
        return Err(SynchronizeNpmGitDependenciesWithNixError::NpmRepoError(
            npm_package.clone(),
            format!("Unrecognized repo URL from the output of npm repo: {npm_repo_output}"),
        ));
    };
    let git_type = captures
        .get(1)
        .ok_or(
            SynchronizeNpmGitDependenciesWithNixError::ParseGitRepositoryError(
                "type".into(),
                npm_repo_output.clone(),
            ),
        )?
        .as_str()
        .to_string();
    let owner = captures
        .get(2)
        .ok_or(
            SynchronizeNpmGitDependenciesWithNixError::ParseGitRepositoryError(
                "owner".into(),
                npm_repo_output.clone(),
            ),
        )?
        .as_str()
        .to_string();
    let repo = captures
        .get(3)
        .ok_or(
            SynchronizeNpmGitDependenciesWithNixError::ParseGitRepositoryError(
                "repo".into(),
                npm_repo_output.clone(),
            ),
        )?
        .as_str()
        .to_string();
    Ok(GitRepo {
        owner,
        repo,
        git_type,
    })
}

fn get_version(
    npm_package: &String,
    rev: &String,
) -> Result<String, SynchronizeNpmGitDependenciesWithNixError> {
    let output = Command::new("npm")
        .args(["show", npm_package, "--json"])
        .stderr(Stdio::null())
        .output()?;
    let npm_version_output = String::from_utf8(output.stdout).map_err(|err| {
        SynchronizeNpmGitDependenciesWithNixError::NpmShowError(
            npm_package.clone(),
            format!("{err:?}"),
        )
    })?;

    let mut package_json_contents: Value = serde_json::from_str(npm_version_output.as_str())?;

    let Some(Value::Array(versions)) = package_json_contents.get_mut("versions") else {
        return Err(SynchronizeNpmGitDependenciesWithNixError::NpmShowError(
            npm_package.clone(),
            format!("No versions returned for npm show"),
        ));
    };

    for version in versions {
        let Value::String(version) = version else {
            continue;
        };
        if version.contains(rev) {
            return Ok(version.clone());
        }
    }

    Err(SynchronizeNpmGitDependenciesWithNixError::NpmShowError(
        npm_package.clone(),
        format!("No version found for the revision {rev}"),
    ))
}
