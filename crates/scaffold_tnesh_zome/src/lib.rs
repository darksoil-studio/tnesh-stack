use anyhow::Result;
use build_fs_tree::dir;
use dialoguer::{theme::ColorfulTheme, Confirm, Input};
use file_tree_utils::{dir_to_file_tree, FileTree, FileTreeError};
use handlebars::RenderError;
use include_dir::{include_dir, Dir};
use serde::{Deserialize, Serialize};
use templates_scaffolding_utils::{
    register_case_helpers, render_template_file_tree_and_merge_with_existing,
    TemplatesScaffoldingUtilsError,
};
use thiserror::Error;

static TEMPLATE: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/template");

#[derive(Error, Debug)]
pub enum ScaffoldTneshZomeError {
    #[error(transparent)]
    RenderError(#[from] RenderError),

    #[error(transparent)]
    TemplatesScaffoldingUtilsError(#[from] TemplatesScaffoldingUtilsError),

    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    DialoguerError(#[from] dialoguer::Error),

    #[error(transparent)]
    FileTreeError(#[from] FileTreeError),
}

#[derive(Serialize, Deserialize, Debug)]
struct ScaffoldTneshZomeData {
    zome_name: String,
    npm_organization: String,
    github_organization: String,
    cachix_cache: Option<String>,
}

pub fn scaffold_tnesh_zome(
    zome_name: Option<String>,
    npm_organization: Option<String>,
    github_organization: Option<String>,
    cachix_cache: Option<String>,
) -> Result<(String, FileTree), ScaffoldTneshZomeError> {
    let zome_name = match zome_name {
        Some(zome_name) => zome_name,
        None => Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter the name of the coordinator zome (eg. posts):")
            .interact_text()?,
    };

    let npm_organization = match npm_organization {
        Some(npm_organization) => npm_organization,
        None => Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter the name of the your NPM organization:")
            .interact_text()?,
    };

    let github_organization = match github_organization {
        Some(github_organization) => github_organization,
        None => Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter the name of your github organization (or github username):")
            .with_initial_text(npm_organization.clone())
            .interact_text()?,
    };

    let cachix_cache = match cachix_cache {
        Some(cachix_cache) => Some(cachix_cache),
        None => {
            let setup_cachix = Confirm::new()
                .with_prompt("Do you want to set up cachix caching of the zome?")
                .interact()?;
            if setup_cachix {
                let cache = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Enter the name of your cachix cache:")
                    .with_initial_text(npm_organization.clone())
                    .interact_text()?;
                Some(cache)
            } else {
                None
            }
        }
    };

    // - Create the src-tauri directory structure
    let template_file_tree = dir_to_file_tree(&TEMPLATE)?;
    println!("filetreee {template_file_tree:?}");
    let h = handlebars::Handlebars::new();
    let h = register_case_helpers(h);

    let existing_file_tree = dir! {};

    let file_tree = render_template_file_tree_and_merge_with_existing(
        existing_file_tree,
        &h,
        &template_file_tree,
        &ScaffoldTneshZomeData {
            zome_name: zome_name.clone(),
            npm_organization,
            github_organization,
            cachix_cache,
        },
    )?;

    Ok((zome_name, file_tree))
}
