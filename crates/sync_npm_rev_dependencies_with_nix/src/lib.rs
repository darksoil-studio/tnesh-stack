use anyhow::Result;
use colored::Colorize;
use ignore::Walk;
use npm_scaffolding_utils::{guess_or_choose_package_manager, PackageManager};
use parse_flake_lock::{FlakeLock, FlakeLockParseError, Node};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{
    env::current_dir,
    fs::File,
    io::BufReader,
    path::PathBuf,
    process::{Command, Stdio},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SynchronizeNpmRevDependenciesWithNixError {
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

    #[error("The flake.lock file is in a directory without parent")]
    FlakeLockHasNoParentError,

    #[error("Error getting the NPM repo for dependency {0}: {1}")]
    NpmRepoError(String, String),

    #[error("Error getting the NPM version for dependency {0}: {1}")]
    NpmShowError(String, String),
}

fn find_flake_lock() -> Result<Option<PathBuf>, SynchronizeNpmRevDependenciesWithNixError> {
    let mut current_dir = current_dir()?;
    let flake_lock = current_dir.join("flake.lock");
    if flake_lock.exists() {
        return Ok(Some(flake_lock));
    }

    while let Some(parent) = current_dir.parent() {
        current_dir = parent.into();
        let flake_lock = current_dir.join("flake.lock");
        if flake_lock.exists() {
            return Ok(Some(flake_lock));
        }
    }

    Ok(None)
}

pub fn synchronize_npm_rev_dependencies_with_nix(
    package_manager: Option<PackageManager>,
) -> Result<(), SynchronizeNpmRevDependenciesWithNixError> {
    // Return silently if no "flake.lock" file exists
    let Some(flake_lock) = find_flake_lock()? else {
        return Ok(());
    };

    let Some(project_root) = flake_lock.parent() else {
        return Err(SynchronizeNpmRevDependenciesWithNixError::FlakeLockHasNoParentError);
    };

    let flake_lock = FlakeLock::new(flake_lock.as_path())?;

    let mut announced = false;
    let mut replaced_some_dep = false;

    for entry in Walk::new(project_root).into_iter().filter_map(|e| e.ok()) {
        let f_name = entry.file_name().to_string_lossy();

        if f_name == "package.json" {
            let mut replaced_some_dep_in_this_file = false;
            let file = File::open(entry.path())?;
            let reader = BufReader::new(file);
            let mut package_json_contents: Value = serde_json::from_reader(reader)?;

            if let Some(Value::Object(deps)) = package_json_contents.get_mut("dependencies") {
                let replaced = sync_deps(&flake_lock, entry.path().into(), deps, announced)?;
                announced = announced || replaced;
                replaced_some_dep = replaced_some_dep || replaced;
                replaced_some_dep_in_this_file = replaced_some_dep_in_this_file || replaced;
            }

            if let Some(Value::Object(deps)) = package_json_contents.get_mut("devDependencies") {
                let replaced = sync_deps(&flake_lock, entry.path().into(), deps, announced)?;
                announced = announced || replaced;
                replaced_some_dep = replaced_some_dep || replaced;
                replaced_some_dep_in_this_file = replaced_some_dep_in_this_file || replaced;
            }

            if replaced_some_dep_in_this_file {
                let st = serde_json::to_string_pretty(&package_json_contents)?;

                std::fs::write(entry.path(), st)?;
            }
        }
    }

    let file_tree = file_tree_utils::load_directory_into_memory(project_root)?;
    let package_manager = match package_manager {
        Some(p) => p,
        None => guess_or_choose_package_manager(&file_tree)?,
    };

    let package_manager_str = package_manager.to_string().to_lowercase();

    if replaced_some_dep {
        println!("");
        println!("Running {package_manager_str} install...");
        Command::new(package_manager_str)
            .arg("install")
            .current_dir(project_root)
            .stdout(Stdio::inherit())
            .output()?;
        println!(
            "
{}",
            "Successfully synchronized npm dependencies with nix".green()
        );
    }

    Ok(())
}

fn sync_deps(
    flake_lock: &FlakeLock,
    package_json_path: PathBuf,
    deps: &mut Map<String, Value>,
    announced: bool,
) -> Result<bool, SynchronizeNpmRevDependenciesWithNixError> {
    let mut replaced_some_dep = false;
    let re = Regex::new(r#"^(.*)-rev\.(.*)$"#)?;

    for (package, dependency_source) in deps {
        if let Value::String(dependency_source_str) = dependency_source.clone() {
            if let Some(captures) = re.captures(&dependency_source_str) {
                let git_repo = get_repo(package)?;
                let _version = captures
                    .get(1)
                    .ok_or(
                        SynchronizeNpmRevDependenciesWithNixError::RevDependencyError(
                            dependency_source_str.clone(),
                        ),
                    )?
                    .as_str();
                let revision = captures
                    .get(2)
                    .ok_or(
                        SynchronizeNpmRevDependenciesWithNixError::RevDependencyError(
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
                            let new_version = get_version(package, &repo_node.locked.rev)?;
                            *dependency_source = Value::String(format!("{new_version}"));

                            if !announced && !replaced_some_dep {
                                println!("");
                                println!(
                                                "Synchronizing npm git dependencies with the upstream nix sources...
"
                                            );
                            }

                            println!(
                                "  - Setting dependency \"{package}\" in file {:?} to rev \"{}\"",
                                package_json_path, new_version
                            );
                            replaced_some_dep = true;
                        }
                    }
                }
            }
        }
    }

    Ok(replaced_some_dep)
}

#[derive(Serialize, Deserialize, Debug)]
struct GitRepo {
    owner: String,
    repo: String,
    git_type: String,
}

fn get_repo(npm_package: &String) -> Result<GitRepo, SynchronizeNpmRevDependenciesWithNixError> {
    let output = Command::new("npm")
        .args(["repo", npm_package, "--no-browser"])
        .stderr(Stdio::null())
        .output()?;
    let npm_repo_output = String::from_utf8(output.stdout).map_err(|err| {
        SynchronizeNpmRevDependenciesWithNixError::NpmRepoError(
            npm_package.clone(),
            format!("{err:?}"),
        )
    })?;

    let re = Regex::new(r#"https://(github|gitlab).com/([^/]+)/([^\n]+)"#)?;
    let Some(captures) = re.captures(&npm_repo_output) else {
        return Err(SynchronizeNpmRevDependenciesWithNixError::NpmRepoError(
            npm_package.clone(),
            format!("Unrecognized repo URL from the output of npm repo: {npm_repo_output}"),
        ));
    };
    let git_type = captures
        .get(1)
        .ok_or(
            SynchronizeNpmRevDependenciesWithNixError::ParseGitRepositoryError(
                "type".into(),
                npm_repo_output.clone(),
            ),
        )?
        .as_str()
        .to_string();
    let owner = captures
        .get(2)
        .ok_or(
            SynchronizeNpmRevDependenciesWithNixError::ParseGitRepositoryError(
                "owner".into(),
                npm_repo_output.clone(),
            ),
        )?
        .as_str()
        .to_string();
    let repo = captures
        .get(3)
        .ok_or(
            SynchronizeNpmRevDependenciesWithNixError::ParseGitRepositoryError(
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
) -> Result<String, SynchronizeNpmRevDependenciesWithNixError> {
    let output = Command::new("npm")
        .args(["show", npm_package, "--json"])
        .stderr(Stdio::null())
        .output()?;
    let npm_version_output = String::from_utf8(output.stdout).map_err(|err| {
        SynchronizeNpmRevDependenciesWithNixError::NpmShowError(
            npm_package.clone(),
            format!("{err:?}"),
        )
    })?;

    let mut package_json_contents: Value = serde_json::from_str(npm_version_output.as_str())?;

    let Some(Value::Array(versions)) = package_json_contents.get_mut("versions") else {
        return Err(SynchronizeNpmRevDependenciesWithNixError::NpmShowError(
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

    Err(SynchronizeNpmRevDependenciesWithNixError::NpmShowError(
        npm_package.clone(),
        format!("No version found for the revision {rev}"),
    ))
}
