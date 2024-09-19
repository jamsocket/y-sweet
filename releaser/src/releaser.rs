use crate::{
    packages::PackageList,
    util::{wrapped_select, BumpType},
};
use anyhow::Result;
use console::style;
use semver::Version;
use std::collections::HashMap;

pub struct Releaser {
    packages: PackageList,
}

impl Releaser {
    pub fn new() -> Self {
        let mut packages = PackageList::new();

        packages.register_cargo_package("y-sweet", "crates/y-sweet");
        // packages.register_node_package("y-sweet", "js-pkg/server");
        // packages.register_node_package("@y-sweet/sdk", "js-pkg/sdk");
        // packages.register_node_package("@y-sweet/client", "js-pkg/client");
        // packages.register_node_package("@y-sweet/react", "js-pkg/react");
        // packages.register_python_package("y_sweet_sdk", "python");

        Releaser { packages }
    }

    pub fn bump(&self, version: Option<Version>) -> Result<()> {
        let mut versions: HashMap<String, Version> = HashMap::new();

        for package in self.packages.iter() {
            let repo_version = package.get_repo_version().unwrap();

            versions.insert(package.name.clone(), repo_version);
        }

        let bump_version = if let Some(version) = version {
            version
        } else {
            let max_version = versions.values().max().unwrap();
            prompt_bump_version(max_version.clone())
        };

        for package in self.packages.iter() {
            package.set_repo_version(&bump_version)?;
        }

        println!("bump version: {}", bump_version);

        Ok(())
    }
}

fn prompt_bump_version(max_version: Version) -> Version {
    println!(
        "Select a version to bump to. The current version is {}",
        style(&max_version).bold().cyan()
    );

    let mut candidate_versions = Vec::new();
    for bump_type in [BumpType::Patch, BumpType::Minor, BumpType::Major] {
        let version = bump_type.bump(max_version.clone());

        candidate_versions.push((version.clone(), format!("{} ({})", bump_type, version)));
    }

    candidate_versions.push((max_version.clone(), "Cancel".to_string()));

    wrapped_select(candidate_versions)
}
