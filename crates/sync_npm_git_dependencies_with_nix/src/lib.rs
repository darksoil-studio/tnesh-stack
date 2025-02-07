use anyhow::Result;
use colored::Colorize;
use ignore::Walk;
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

    #[error("Error parsing git {0} for dependency {1}")]
    ParseGitRepositoryError(String, String),

    #[error("Error parsing the rev-tag. dependency {0}")]
    RevTagDependencyError(String),

    #[error("Error getting the NPM repo for dependency {0}: {1}")]
    NpmRepoError(String, String),
}

pub fn synchronize_npm_git_dependencies_with_nix(
) -> Result<(), SynchronizeNpmGitDependenciesWithNixError> {
    let flake_lock = current_dir()?.join("flake.lock");

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
            let file = File::open(entry.path())?;
            let reader = BufReader::new(file);
            let mut package_json_contents: Value = serde_json::from_reader(reader)?;

            if let Some(Value::Object(deps)) = package_json_contents.get_mut("dependencies") {
                let re = Regex::new(r#"rev-tag\.(.*)$"#)?;

                for (package, dependency_source) in deps {
                    if let Value::String(dependency_source_str) = dependency_source.clone() {
                        if let Some(captures) = re.captures(&dependency_source_str) {
                            let git_repo = get_repo(package, &dependency_source_str)?;
                            let revision= captures
                                .get(1)
                                .ok_or(
                                    SynchronizeNpmGitDependenciesWithNixError::RevTagDependencyError(dependency_source_str.clone())
)?
                                .as_str();

                            for root_node in flake_lock.root.values() {
                                if let Node::Repo(repo_node) = root_node {
                                    if repo_node.locked.node_type == git_repo.git_type
                                        && repo_node.locked.owner == git_repo.owner
                                        && repo_node.locked.repo == git_repo.repo
                                        && revision != repo_node.locked.rev
                                    {
                                        *dependency_source = Value::String(format!(
                                            "rev-tag.{}",
                                            repo_node.locked.rev
                                        ));

                                        if !announced {
                                            announced = true;
                                            println!("");
                                            println!(
                                        "Synchronizing npm git dependencies with the upstream nix sources..."
                                    );
                                        }

                                        println!(
                                    "  - Setting dependency \"{package}\" in file {:?} to rev \"{}\"",
                                    entry.path(), repo_node.locked.rev
                                );
                                        replaced_some_dep = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            let st = serde_json::to_string_pretty(&package_json_contents)?;

            std::fs::write(entry.path(), st)?;
        }
    }

    if replaced_some_dep {
        println!("Running pnpm install...");
        println!("");
        Command::new("pnpm")
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

fn get_repo(
    npm_package: &String,
    tag: &String,
) -> Result<GitRepo, SynchronizeNpmGitDependenciesWithNixError> {
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
