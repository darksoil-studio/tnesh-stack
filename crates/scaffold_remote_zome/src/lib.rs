use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Select};
use file_tree_utils::{
    find_files_by_extension, find_files_by_name, insert_file, map_all_files, map_file, FileTree, FileTreeError
};
use holochain_types::prelude::{
    DnaManifest, DnaManifestCurrentBuilder, ZomeDependency, ZomeLocation,
};
use nix_scaffolding_utils::{add_flake_input, NixScaffoldingUtilsError};
use npm_scaffolding_utils::{
     add_npm_dependency_to_package,  get_npm_package_name,
    NpmScaffoldingUtilsError,
};
use path_clean::PathClean;
use regex::{Captures, Regex};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScaffoldRemoteZomeError {
    #[error(transparent)]
    NpmScaffoldingUtilsError(#[from] NpmScaffoldingUtilsError),

    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    RegexError(#[from] regex::Error),

    #[error(transparent)]
    NixScaffoldingUtilsError(#[from] NixScaffoldingUtilsError),

    #[error(transparent)]
    DialoguerError(#[from] dialoguer::Error),

    #[error(transparent)]
    FileTreeError(#[from] FileTreeError),

    #[error("No nixified DNAs were found in this project.")]
    NoDnasFoundError,

    #[error("custom_element and custom_element_import must either both be set or none be set.")]
    ContextElementOrImportError,

    #[error("The dna {0} was not found in this project.")]
    DnaNotFoundError(String),

    #[error("No \"zomes = {{\" code property was found in the nix file {0:?}.")]
    ZomesPropertyNotFoundError(PathBuf),

    #[error("No integrity or coordinator zomes were specified to be scaffolded.")]
    NoZomesSpecifiedError,

    #[error(transparent)]
    SerdeYamlError(#[from] serde_yaml::Error),

    #[error("Malformed DNA manifest at {0}: {1}")]
    MalformedDnaManifest(PathBuf, String),
}

pub fn scaffold_remote_zome(
    file_tree: FileTree,
    module_name: String,
    integrity_zome_name: Option<String>,
    coordinator_zome_name: Option<String>,
    remote_zome_git_url: String,
    remote_zome_git_branch: Option<String>,
    remote_npm_package_name: String,
    remote_npm_package_path: PathBuf,
    local_dna_to_add_the_zome_to: Option<String>,
    local_npm_package_to_add_the_ui_to: Option<String>,
    context_element: Option<String>,
    context_element_import: Option<String>,
) -> Result<FileTree, ScaffoldRemoteZomeError> {
    let nix_git_url = format!(
        "{remote_zome_git_url}{}",
        remote_zome_git_branch
            .clone()
            .map(|b| format!("/{b}"))
            .unwrap_or_default()
    );
    let mut file_tree = add_flake_input(file_tree, module_name.clone(), nix_git_url.clone())?;

    let dna = get_or_choose_dna(&file_tree, &module_name, local_dna_to_add_the_zome_to)?;

    add_zome_to_nixified_dna(
        &mut file_tree,
        dna.clone(),
        &module_name,
        integrity_zome_name,
        coordinator_zome_name,
    )?;

    let npm_dependency_source = format!(
        "{remote_zome_git_url}{}&path:{}",
        remote_zome_git_branch
            .map(|b| format!("#{b}"))
            .unwrap_or_default(),
        remote_npm_package_path.to_str().unwrap()
    );

    let (mut file_tree , package_json)= add_npm_dependency(
        file_tree, 
        remote_npm_package_name, 
        npm_dependency_source, 
        local_npm_package_to_add_the_ui_to
    )?;

    match (context_element, context_element_import) {
        (Some(context_element), Some(context_element_import)) => {
          file_tree = add_context_element(
              file_tree,
              dna.name,
               &package_json, 
               context_element, 
               context_element_import
           )?;
        }
        (None, None)=> {
            
        },
        _ => {
            return Err(ScaffoldRemoteZomeError::ContextElementOrImportError);
        }
    }

    Ok(file_tree)
}

fn select_npm_package(npm_dependency_package_name: String, npm_packages: Vec<String>) -> Result<usize, NpmScaffoldingUtilsError> {
    let mut i = 0;
    let mut found = false;

    while !found {
        let package = &npm_packages[i];
        if !(package.ends_with("-dev") || package.contains("test")) {
            found = true;
        } else {
            i += 1;
        }
    }

    let default = i;
    
    Ok(Select::with_theme(&ColorfulTheme::default())
        .with_prompt(
        format!(    
"Multiple NPM packages were found in this project. Choose one to which to add the {npm_dependency_package_name} dependency:"))
        .default(default)
        .items(&npm_packages[..])
        .interact()?)
}

pub fn add_npm_dependency(
    mut file_tree: FileTree,
    dependency: String,
    dependency_source: String,
    package_to_add_the_dependency_to: Option<String>,
) -> Result<(FileTree, (PathBuf, String)), NpmScaffoldingUtilsError> {
    let mut package_jsons = find_files_by_name(&file_tree, PathBuf::from("package.json").as_path());

    let package_json = match package_jsons.len() {
        0 => Err(NpmScaffoldingUtilsError::NoNpmPackagesFoundError)?,
        1 => {
            let package_json = package_jsons.pop_first().unwrap();
            map_file(
                &mut file_tree,
                package_json.0.as_path(),
                |_package_json_content| {
                    add_npm_dependency_to_package(&package_json, &dependency, &dependency_source)
                },
            )?;
            println!("Added dependency {dependency} to {:?}.", package_json.0);
            package_json
        }
        _ => {
            let package_jsons: Vec<(PathBuf, String)> = package_jsons.into_iter().collect();
            let packages_names = package_jsons
                .iter()
                .map(|package_json| get_npm_package_name(package_json))
                .collect::<Result<Vec<String>, NpmScaffoldingUtilsError>>()?;

            let package_index = match package_to_add_the_dependency_to {
                Some(package_to_add_to) => packages_names
                    .iter()
                    .position(|package_name| package_name.eq(&package_to_add_to))
                    .ok_or(NpmScaffoldingUtilsError::NpmPackageNotFoundError(
                        package_to_add_to.clone(),
                    ))?,
                None => {
                    let default_ui_package_json_index = package_jsons
                        .iter()
                        .position(|(path, _)| path.eq(&PathBuf::from("ui/package.json")));

                    if let Some(default_ui_package_index) = default_ui_package_json_index {
                        default_ui_package_index
                    } else {
                        select_npm_package(dependency.clone(), packages_names)?
                    }
                }
            };

            let package_json = &package_jsons[package_index];

            map_file(
                &mut file_tree,
                package_json.0.as_path(),
                |_package_json_content| {
                    add_npm_dependency_to_package(package_json, &dependency, &dependency_source)
                },
            )?;
            println!("Added dependency {dependency} to {:?}.", package_json.0);
            package_json.clone()
        }
    };

    Ok((file_tree, package_json))
}

fn add_context_element(
    mut file_tree: FileTree,
    dna_role_name: String,
    npm_package: &(PathBuf, String),
    context_element: String,
    context_element_import: String,
) -> Result<FileTree, ScaffoldRemoteZomeError> {
    let mut found = false;

    let mut npm_package_folder = npm_package.0.clone();
    npm_package_folder.pop();

    let context_re = Regex::new(
        r"(?<before>[\S\s]*)<app-client-context(?<appclientcontextprops>[^>]*)>(?<middle>[\S\s]*)</app-client-context>(?<after>[\S\s]*)"
    )?;
    let indent_before_context_re = Regex::new(
        r"\n(?<indent>[\s]*)<app-client-context"
    )?;
    let import_re = Regex::new(
        r"(?<before>[\S\s]*)import (?<importmiddle>[^\n;]*)[;|\n](?<after>[\S\s]*)"
    )?;

    map_all_files(&mut file_tree, |path,contents| {
        if !path.starts_with(&npm_package_folder) {
            return Ok(contents);
        }

        if context_re.is_match(&contents) {
            let indent_captures = indent_before_context_re.captures(&contents);
            let indentation = match indent_captures {
                Some(c) => c["indent"].to_string(),
                None => "  ".to_string()
            };

            let new_contents = context_re.replace(&contents, |caps: &Captures| {
                let middle = caps["middle"].replace('\n', "\n  ");
                format!(
                    r#"{}<app-client-context{}>
{indentation}  <{context_element} role="{dna_role_name}">{middle}</{context_element}>
{indentation}</app-client-context>{}"#,
                    &caps["before"], &caps["appclientcontextprops"], &caps["after"],
                )
            });
            let new_contents = import_re.replace(&new_contents, |caps: &Captures| {
                format!(
                    r#"{}import {};
import '{}';{}"#,
                    &caps["before"], &caps["importmiddle"], &context_element_import, &caps["after"],
                )
            });
            
            Ok(new_contents.to_string())
        }  else {
            Ok(contents)
        }
    } )?;

    
    Ok(file_tree)
}

#[derive(Debug, Clone)]
pub struct NixifiedDna {
    name: String,
    dna_nix: (PathBuf, String),
    dna_manifest: (PathBuf, String),
    dna_manifest_reference_index: usize,
}

fn add_zome_to_nixified_dna(
    file_tree: &mut FileTree,
    nixified_dna: NixifiedDna,
    module_name: &String,
    integrity_zome_name: Option<String>,
    coordinator_zome_name: Option<String>,
) -> Result<(), ScaffoldRemoteZomeError> {
    if integrity_zome_name.is_none() && coordinator_zome_name.is_none() {
        return Err(ScaffoldRemoteZomeError::NoZomesSpecifiedError);
    }

    let integrity_string_to_add = integrity_zome_name
        .clone()
        .map(|name| format!("\n          {name} = inputs'.{module_name}.packages.{name};"))
        .unwrap_or_default();
    let coordinator_string_to_add = coordinator_zome_name
        .clone()
        .map(|name| format!("\n          {name} = inputs'.{module_name}.packages.{name};"))
        .unwrap_or_default();
    let string_to_add = format!("{integrity_string_to_add}{coordinator_string_to_add}");

    map_file(
        file_tree,
        &nixified_dna.dna_nix.0.clone(),
        |mut dna_contents| {
            let zomes_re = Regex::new(r#"zomes = \{"#)?;

            let captures_iter: Vec<Captures<'_>> = zomes_re.captures_iter(&dna_contents).collect();

            match captures_iter.len() {
                0 => {
                    return Err(ScaffoldRemoteZomeError::ZomesPropertyNotFoundError(
                        nixified_dna.dna_nix.0.clone(),
                    ));
                }
                1 => Ok(zomes_re
                    .replace(&dna_contents, format!("zomes = {{{string_to_add}"))
                    .to_string()),
                _ => {
                    let distance_to_dna_manifest = |captures: &Captures<'_>| {
                        ((nixified_dna.dna_manifest_reference_index as isize)
                            - captures.get(0).unwrap().start() as isize)
                            .abs() as usize
                    };
                    let captures = captures_iter
                        .into_iter()
                        .min_by_key(distance_to_dna_manifest)
                        .unwrap();

                    dna_contents.insert_str(captures.get(0).unwrap().end(), &string_to_add);

                    Ok(dna_contents)
                }
            }
        },
    )?;

    println!(
        "Added the integrity zome {integrity_zome_name:?} and the coordinator zome {coordinator_zome_name:?} to {:?}.", 
        nixified_dna.dna_nix.0
    );

    let dna_manifest: DnaManifest = serde_yaml::from_str(nixified_dna.dna_manifest.1.as_str())?;

    let (mut integrity_manifest, mut coordinator_manifest) = match dna_manifest.clone() {
        DnaManifest::V1(m) => (m.integrity, m.coordinator),
    };
    if let Some(integrity_zome) = integrity_zome_name.clone() {
        integrity_manifest
            .zomes
            .push(holochain_types::prelude::ZomeManifest {
                name: integrity_zome.into(),
                hash: None,
                location: ZomeLocation::Bundled(PathBuf::from("<NIX_PACKAGE>")),
                dependencies: None,
                dylib: None,
            });
    }
    if let Some(coordinator_zome) = coordinator_zome_name.clone() {
        coordinator_manifest
            .zomes
            .push(holochain_types::prelude::ZomeManifest {
                name: coordinator_zome.into(),
                hash: None,
                location: ZomeLocation::Bundled(PathBuf::from("<NIX_PACKAGE>")),
                dependencies: integrity_zome_name
                    .clone()
                    .map(|name| vec![ZomeDependency { name: name.into() }]),
                dylib: None,
            });
    }

    let new_manifest: DnaManifest = DnaManifestCurrentBuilder::default()
        .coordinator(coordinator_manifest)
        .integrity(integrity_manifest)
        .name(dna_manifest.name())
        .build()
        .unwrap()
        .into();

    insert_file(
        file_tree,
        &nixified_dna.dna_manifest.0,
        &serde_yaml::to_string(&new_manifest)?,
    )?;

    println!(
        "Added the integrity zome {integrity_zome_name:?} and the coordinator zome {coordinator_zome_name:?} to {:?}.", 
        nixified_dna.dna_manifest.0
    );

    Ok(())
}

fn get_or_choose_dna(
    file_tree: &FileTree,
    module_name: &String,
    local_dna_to_add_the_zome_to: Option<String>,
) -> Result<NixifiedDna, ScaffoldRemoteZomeError> {
    let nixified_dnas = find_nixified_dnas(&file_tree)?;

    match nixified_dnas.len() {
        0 => Err(ScaffoldRemoteZomeError::NoDnasFoundError)?,
        1 => {
            let nixified_dna = nixified_dnas.first().unwrap();
            if let Some(local_dna) = local_dna_to_add_the_zome_to {
                if !nixified_dna.name.eq(&local_dna) {
                    return Err(ScaffoldRemoteZomeError::NoDnasFoundError);
                }
            }
            Ok(nixified_dna.clone())
        }
        _ => {
            let dna_names: Vec<String> = nixified_dnas.iter().map(|dna| dna.name.clone()).collect();

            let dna_index = match local_dna_to_add_the_zome_to{
                Some(local_dna) => dna_names
                    .iter()
                    .position(|dna_name| dna_name.eq(&local_dna))
                    .ok_or(NpmScaffoldingUtilsError::NpmPackageNotFoundError(
                        local_dna.clone(),
                    ))?,
                None => {
                    Select::with_theme(&ColorfulTheme::default())
                    .with_prompt(format!("Multiple DNAs were found in this repository, choose one to scaffold the {module_name} zome into:",
                    ))
                    .default(0)
                    .items(&dna_names[..])
                    .interact()?},
            };

            let nixified_dna = &nixified_dnas[dna_index];
            Ok(nixified_dna.clone())
        }
    }
}

fn find_nixified_dnas(file_tree: &FileTree) -> Result<Vec<NixifiedDna>, ScaffoldRemoteZomeError> {
    // Find all dna.yaml files
    // Go through all nix files, finding `dnaManifest = `
    // For each DNA, find the ones that are referred to from the nix files, those are the eligible DNAs
    // Have the user select one of them
    // Add the zome in the DNA manifest
    // Add the zome in the nix file, in zomes = {
    let dna_manifest_re = Regex::new(r#"dnaManifest = (?<dnaManifestPath>./[^;]*);"#)?;

    let dna_yaml_files = find_files_by_name(file_tree, PathBuf::from("dna.yaml").as_path());

    let mut nixified_dnas: Vec<NixifiedDna> = Vec::new();

    let nix_files = find_files_by_extension(file_tree, "nix");

    for (dna_manifest_path, dna_manifest_content) in dna_yaml_files {
        let value: serde_yaml::Value = serde_yaml::from_str(dna_manifest_content.as_str())?;

        let serde_yaml::Value::Mapping(mapping) = value else {
            return Err(ScaffoldRemoteZomeError::MalformedDnaManifest(
                dna_manifest_path,
                String::from("root level object is not a mapping"),
            ));
        };

        let Some(serde_yaml::Value::String(dna_name)) = mapping.get("name") else {
            return Err(ScaffoldRemoteZomeError::MalformedDnaManifest(
                dna_manifest_path,
                String::from("manifest does not have a name"),
            ));
        };

        for (nix_file_path, nix_file_contents) in nix_files.iter() {
            if let Some(captures) = dna_manifest_re.captures(&nix_file_contents) {
                let capture = captures.name("dnaManifestPath").unwrap();
                let dna_manifest_path_in_nix_file = capture.as_str();

                let mut nix_file_folder = nix_file_path.clone();
                nix_file_folder.pop();

                let full_path = nix_file_folder.join(dna_manifest_path_in_nix_file).clean();

                if full_path.eq(&dna_manifest_path) {
                    nixified_dnas.push(NixifiedDna {
                        name: dna_name.clone(),
                        dna_nix: (nix_file_path.clone(), nix_file_contents.clone()),
                        dna_manifest: (dna_manifest_path.clone(), dna_manifest_content.clone()),
                        dna_manifest_reference_index: capture.start(),
                    });
                }
            }
        }
    }

    Ok(nixified_dnas)
}

#[cfg(test)]
mod tests {
    use super::*;
    use build_fs_tree::{dir, file};
    use file_tree_utils::file_content;
    use pretty_assertions::assert_eq;

    #[test]
    fn multiple_package_test() {
        let repo: FileTree = dir! {
            "flake.nix" => file!(default_flake_nix()),
            "dna.nix" => file!(empty_dna_nix()),
            "workdir" => dir! {
                "dna.yaml" => file!(empty_dna_yaml("another_dna"))
            },
            "dna.yaml" => file!(empty_dna_yaml("my_dna")),
            "package.json" => file!(empty_package_json("root")),
            "packages" => dir! {
                "package1" => dir! {
                    "package.json" => file!(empty_package_json("package1")),
                    "app.js" => file!(empty_app_js())
                },
                "package2" => dir! {
                    "package.json" => file!(empty_package_json("package2"))
                }
            }
        };

        let repo = scaffold_remote_zome(
            repo,
            "profiles-zome".into(),
            Some("profiles_integrity".into()),
            Some("profiles".into()),
            "github:darksoil-studio/profiles-zome".into(),
            Some("main-0.3".into()),
            "@darksoil-studio/profiles-zome".into(),
            PathBuf::from("ui"),
            None,
            Some("package1".into()),
            Some("profiles-context".into()),
            Some("@darksoil-studio/profiles-zome/dist/elements/profiles-context.js".into()),
        )
        .unwrap();

        assert_eq!(
            file_content(
                &repo,
                PathBuf::from("packages/package1/package.json").as_path()
            )
            .unwrap(),
            r#"{
  "name": "package1",
  "dependencies": {
    "@darksoil-studio/profiles-zome": "github:darksoil-studio/profiles-zome#main-0.3&path:ui"
  }
}"#
        );

        assert_eq!(
            file_content(&repo, PathBuf::from("dna.yaml").as_path()).unwrap(),
            r#"manifest_version: '1'
name: my_dna
integrity:
  network_seed: null
  properties: null
  origin_time: 1709638576394039
  zomes:
  - name: profiles_integrity
    hash: null
    bundled: <NIX_PACKAGE>
    dependencies: null
    dylib: null
coordinator:
  zomes:
  - name: profiles
    hash: null
    bundled: <NIX_PACKAGE>
    dependencies:
    - name: profiles_integrity
    dylib: null
"#
        );

        assert_eq!(
            file_content(&repo, PathBuf::from("flake.nix").as_path()).unwrap(),
            r#"{
  description = "Template for Holochain app development";
  
  inputs = {
    profiles-zome.url = "github:darksoil-studio/profiles-zome/main-0.3";
    nixpkgs.follows = "holonix/nixpkgs";

    holonix.url = "github:holochain/holonix";
    tnesh-stack.url = "github:darksoil-studio/tnesh-stack/main-0.3";
  };

  outputs = inputs @ { ... }:
    inputs.holonix.inputs.flake-parts.lib.mkFlake
    {
      inherit inputs;
    }
    {
      imports = [
        ./dna.nix
      ];

      systems = builtins.attrNames inputs.holonix.devShells;
      perSystem =
        { inputs'
        , config
        , pkgs
        , system
        , lib
        , self'
        , ...
        }: {
          devShells.default = pkgs.mkShell {
            inputsFrom = [ 
              inputs'.tnesh-stack.devShells.synchronized-pnpm
              inputs'.holonix.devShells.default
            ];
          };
        };
    };
}
"#
        );

        assert_eq!(
            file_content(&repo, PathBuf::from("dna.nix").as_path()).unwrap(),
            r#"{ inputs, ... }:

{
  perSystem =
    { inputs'
    , self'
    , system
    , ...
    }: {
  	  packages.my_dna = inputs.tnesh-stack.outputs.builders.${system}.dna {
        dnaManifest = ./dna.yaml;
        zomes = {
          profiles_integrity = inputs'.profiles-zome.packages.profiles_integrity;
          profiles = inputs'.profiles-zome.packages.profiles;
        };
      };

  	  packages.another_dna = inputs.tnesh-stack.outputs.builders.${system}.dna {
        dnaManifest = ./workdir/dna.yaml;
        zomes = {
        };
      };
    };
}
"#
        );

        assert_eq!(
            file_content(&repo, PathBuf::from("packages/package1/app.js").as_path()).unwrap(),
            r#"import '@tnesh-stack/elements/dist/elements/app-client-context.js';
import '@darksoil-studio/profiles-zome/dist/elements/profiles-context.js';

export class App {

  render() {
    return html`
      <app-client-context .client=${this.client}>
        <profiles-context role="my_dna">
          <linked-devices-context role="my_dna">
          </linked-devices-context>
        </profiles-context>
      </app-client-context>
    `;
  }
}"#
        );
    }
    
    #[test]
    fn single_package_test() {
        let repo: FileTree = dir! {
            "flake.nix" => file!(default_flake_nix()),
            "dna.nix" => file!(empty_dna_nix()),
            "workdir" => dir! {
                "dna.yaml" => file!(empty_dna_yaml("another_dna"))
            },
            "dna.yaml" => file!(empty_dna_yaml("my_dna")),
            "package.json" => file!(empty_package_json("root")),
            "ui" => dir! {
                "package.json" => file!(empty_package_json("package1")),
                "app.js" => file!(empty_app_js())
            }
        };

        let repo = scaffold_remote_zome(
            repo,
            "profiles-zome".into(),
            Some("profiles_integrity".into()),
            Some("profiles".into()),
            "github:darksoil-studio/profiles-zome".into(),
            Some("main-0.3".into()),
            "@darksoil-studio/profiles-zome".into(),
            PathBuf::from("ui"),
            None,
            None,
            Some("profiles-context".into()),
            Some("@darksoil-studio/profiles-zome/dist/elements/profiles-context.js".into()),
        )
        .unwrap();

        assert_eq!(
            file_content(
                &repo,
                PathBuf::from("ui/package.json").as_path()
            )
            .unwrap(),
            r#"{
  "name": "package1",
  "dependencies": {
    "@darksoil-studio/profiles-zome": "github:darksoil-studio/profiles-zome#main-0.3&path:ui"
  }
}"#
        );

        assert_eq!(
            file_content(&repo, PathBuf::from("dna.yaml").as_path()).unwrap(),
            r#"manifest_version: '1'
name: my_dna
integrity:
  network_seed: null
  properties: null
  origin_time: 1709638576394039
  zomes:
  - name: profiles_integrity
    hash: null
    bundled: <NIX_PACKAGE>
    dependencies: null
    dylib: null
coordinator:
  zomes:
  - name: profiles
    hash: null
    bundled: <NIX_PACKAGE>
    dependencies:
    - name: profiles_integrity
    dylib: null
"#
        );

        assert_eq!(
            file_content(&repo, PathBuf::from("flake.nix").as_path()).unwrap(),
            r#"{
  description = "Template for Holochain app development";
  
  inputs = {
    profiles-zome.url = "github:darksoil-studio/profiles-zome/main-0.3";
    nixpkgs.follows = "holonix/nixpkgs";

    holonix.url = "github:holochain/holonix";
    tnesh-stack.url = "github:darksoil-studio/tnesh-stack/main-0.3";
  };

  outputs = inputs @ { ... }:
    inputs.holonix.inputs.flake-parts.lib.mkFlake
    {
      inherit inputs;
    }
    {
      imports = [
        ./dna.nix
      ];

      systems = builtins.attrNames inputs.holonix.devShells;
      perSystem =
        { inputs'
        , config
        , pkgs
        , system
        , lib
        , self'
        , ...
        }: {
          devShells.default = pkgs.mkShell {
            inputsFrom = [ 
              inputs'.tnesh-stack.devShells.synchronized-pnpm
              inputs'.holonix.devShells.default
            ];
          };
        };
    };
}
"#
        );

        assert_eq!(
            file_content(&repo, PathBuf::from("dna.nix").as_path()).unwrap(),
            r#"{ inputs, ... }:

{
  perSystem =
    { inputs'
    , self'
    , system
    , ...
    }: {
  	  packages.my_dna = inputs.tnesh-stack.outputs.builders.${system}.dna {
        dnaManifest = ./dna.yaml;
        zomes = {
          profiles_integrity = inputs'.profiles-zome.packages.profiles_integrity;
          profiles = inputs'.profiles-zome.packages.profiles;
        };
      };

  	  packages.another_dna = inputs.tnesh-stack.outputs.builders.${system}.dna {
        dnaManifest = ./workdir/dna.yaml;
        zomes = {
        };
      };
    };
}
"#
        );

        assert_eq!(
            file_content(&repo, PathBuf::from("ui/app.js").as_path()).unwrap(),
            r#"import '@tnesh-stack/elements/dist/elements/app-client-context.js';
import '@darksoil-studio/profiles-zome/dist/elements/profiles-context.js';

export class App {

  render() {
    return html`
      <app-client-context .client=${this.client}>
        <profiles-context role="my_dna">
          <linked-devices-context role="my_dna">
          </linked-devices-context>
        </profiles-context>
      </app-client-context>
    `;
  }
}"#
        );
    }

    fn empty_package_json(package_name: &str) -> String {
        format!(
            r#"{{
  "name": "{package_name}",
  "dependencies": {{}}
}}
"#
        )
    }

    fn empty_dna_yaml(dna_name: &str) -> String {
        format!(
            r#"
---
manifest_version: "1"
name: {dna_name}
integrity:
  network_seed: ~
  properties: ~
  origin_time: 1709638576394039
  zomes: []
coordinator:
  zomes: []
"#
        )
    }

    fn default_flake_nix() -> String {
        String::from(
            r#"{
  description = "Template for Holochain app development";
  
  inputs = {
    nixpkgs.follows = "holonix/nixpkgs";

    holonix.url = "github:holochain/holonix";
    tnesh-stack.url = "github:darksoil-studio/tnesh-stack/main-0.3";
  };

  outputs = inputs @ { ... }:
    inputs.holonix.inputs.flake-parts.lib.mkFlake
    {
      inherit inputs;
    }
    {
      imports = [
        ./dna.nix
      ];

      systems = builtins.attrNames inputs.holonix.devShells;
      perSystem =
        { inputs'
        , config
        , pkgs
        , system
        , lib
        , self'
        , ...
        }: {
          devShells.default = pkgs.mkShell {
            inputsFrom = [ 
              inputs'.tnesh-stack.devShells.synchronized-pnpm
              inputs'.holonix.devShells.default
            ];
          };
        };
    };
}
"#,
        )
    }

    fn empty_dna_nix() -> String {
        String::from(
            r#"{ inputs, ... }:

{
  perSystem =
    { inputs'
    , self'
    , system
    , ...
    }: {
  	  packages.my_dna = inputs.tnesh-stack.outputs.builders.${system}.dna {
        dnaManifest = ./dna.yaml;
        zomes = {
        };
      };

  	  packages.another_dna = inputs.tnesh-stack.outputs.builders.${system}.dna {
        dnaManifest = ./workdir/dna.yaml;
        zomes = {
        };
      };
    };
}
"#,
        )
    }

    fn empty_app_js() -> String {
        r#"import '@tnesh-stack/elements/dist/elements/app-client-context.js';

export class App {

  render() {
    return html`
      <app-client-context .client=${this.client}>
        <linked-devices-context role="my_dna">
        </linked-devices-context>
      </app-client-context>
    `;
  }
}"#
        .into()
    }

    fn empty_svelte_app() -> String {
        r#"<script>
import { AppClient } from '@holochain/client';
</script>
<app-client-context client={this.client}>
  <linked-devices-context role="my_dna">
  </linked-devices-context>
</app-client-context>
"#
        .into()
    }

    #[test]
    fn add_context_to_svelte_app() {
        let repo: FileTree = dir! {
            "package.json" => file!(empty_package_json("package1")),
            "app.svelte" => file!(empty_svelte_app()),
        };
        let result = add_context_element(repo, 
            "my_dna".into(),
            &(PathBuf::from("package.json"), empty_package_json("package1")), 
            "profiles-context".into(),
             "@darksoil-studio/profiles-zome/dist/elements/profiles-context.js".into()
         ).unwrap();

        assert_eq!(
            file_content(&result, PathBuf::from("app.svelte").as_path()).unwrap(),
            r#"<script>
import { AppClient } from '@holochain/client';
import '@darksoil-studio/profiles-zome/dist/elements/profiles-context.js';
</script>
<app-client-context client={this.client}>
  <profiles-context role="my_dna">
    <linked-devices-context role="my_dna">
    </linked-devices-context>
  </profiles-context>
</app-client-context>
"#
        );
    }
}
