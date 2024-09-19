use crate::{
    git::Git,
    package_manager::PackageType,
    packages::PackageList,
    util::{get_root_dir, wrapped_select, BumpType},
};
use anyhow::{Context, Result};
use console::style;
use dialoguer::Confirm;
use semver::Version;
use std::collections::HashMap;

pub struct Releaser {
    packages: PackageList,
    git: Git,
}

pub fn ensure_repo_ready(git: &Git) -> Result<()> {
    if !git.clean()? {
        anyhow::bail!("Repository is not clean");
    }

    let branch = git.get_branch()?;
    if branch != "main" {
        anyhow::bail!("Git branch is not main");
    }

    Ok(())
}

impl Releaser {
    pub fn new(packages: PackageList) -> Self {
        let root_dir = get_root_dir();
        let git = Git::new(&root_dir).unwrap();
        Releaser { packages, git }
    }

    pub fn bump(&self, version: Option<Version>) -> Result<()> {
        ensure_repo_ready(&self.git)?;

        let mut versions: HashMap<String, Version> = HashMap::new();
        let mut deps: HashMap<PackageType, Vec<String>> = HashMap::new();

        for package in self.packages.iter() {
            let repo_version = package.get_repo_version().unwrap();

            versions.insert(package.name.clone(), repo_version);
            deps.entry(package.package_type.clone())
                .or_insert_with(Vec::new)
                .push(package.name.clone());
        }

        let bump_version = if let Some(version) = version {
            version
        } else {
            let max_version = versions.values().max().unwrap();
            prompt_bump_version(max_version.clone())
        };

        for package in self.packages.iter() {
            let old_version = &versions[&package.name];
            if old_version < &bump_version {
                println!(
                    "Bumping {} package {} from {} to {}",
                    style(&package.package_type).bold().red(),
                    style(&package.name).bold().cyan(),
                    style(&old_version).bold().magenta(),
                    style(&bump_version).bold().green()
                );
                package.set_repo_version(&bump_version)?;
            } else {
                println!(
                    "{} package {} is already at {}",
                    style(&package.package_type).bold().red(),
                    style(&package.name).bold().cyan(),
                    style(&bump_version).bold().yellow()
                );
            }
        }

        // Bump dependencies
        for package in self.packages.iter() {
            let package_type = package.package_type;
            let Some(deps) = deps.get(&package_type) else {
                continue;
            };
            println!(
                "Updating dependencies for {} package {}",
                style(&package.package_type).bold().red(),
                style(&package.name).bold().cyan()
            );
            let result = package
                .package_type
                .get_package_manager()
                .update_dependencies(&package.path, deps, &bump_version)
                .context("Updating dependencies")?;
            if result {
                println!(
                    "Updated dependencies for {} package {}",
                    style(&package.package_type).bold().red(),
                    style(&package.name).bold().cyan()
                );
            }
        }

        // Update lockfiles
        for package in self.packages.iter() {
            if package.package_type == PackageType::Python {
                continue;
            }
            println!(
                "Updating lockfile for {} package {}",
                style(&package.package_type).bold().red(),
                style(&package.name).bold().cyan()
            );
            package
                .package_type
                .get_package_manager()
                .update_lockfile(&package.path)?;
        }

        // Prompt to confirm commit to git.
        let confirm = Confirm::new()
            .with_prompt("Commit changes to git?")
            .default(false)
            .interact()?;
        if !confirm {
            return Ok(());
        }

        // Check out a branch
        let branch_name = format!("release/{}", bump_version);
        self.git.checkout_new_branch(&branch_name)?;

        // Commit changes
        self.git
            .commit_all(&format!("Bump version to {}", bump_version))?;

        // Push changes
        self.git.push()?;

        println!(
            "Pushed to Git. To create a PR, visit https://github.com/jamsocket/y-sweet/pull/new/{}",
            branch_name
        );

        Ok(())
    }
}

fn prompt_bump_version(max_version: Version) -> Version {
    println!(
        "Select a version to bump to. The current version is {}",
        style(&max_version).bold().magenta()
    );

    let mut candidate_versions = Vec::new();
    for bump_type in [BumpType::Patch, BumpType::Minor, BumpType::Major] {
        let version = bump_type.bump(max_version.clone());

        candidate_versions.push((version.clone(), format!("{} ({})", bump_type, version)));
    }

    candidate_versions.push((max_version.clone(), "Cancel".to_string()));

    wrapped_select(candidate_versions)
}
