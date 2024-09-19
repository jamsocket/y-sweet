use clap::{Parser, Subcommand};
use console::style;
use packages::PackageList;
use semver::Version;
use std::collections::HashMap;
use util::{wrapped_select, BumpType};

mod package_manager;
mod packages;
mod util;

#[derive(Parser)]
#[clap(version = "0.1.0", author = "Y-sweet")]
struct Args {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Bump {
        /// The version to bump to.
        #[clap(long)]
        version: Option<Version>,
    },
}

struct Releaser {
    packages: PackageList,
}

impl Releaser {
    pub fn new() -> Self {
        let mut packages = PackageList::new();

        packages.register_cargo_package("y-sweet", "crates/y-sweet");
        packages.register_node_package("y-sweet", "js-pkg/server");
        packages.register_node_package("@y-sweet/sdk", "js-pkg/sdk");
        packages.register_node_package("@y-sweet/client", "js-pkg/client");
        packages.register_node_package("@y-sweet/react", "js-pkg/react");
        packages.register_python_package("y_sweet_sdk", "python");

        Releaser { packages }
    }

    pub fn bump(&self, version: Option<Version>) {
        let mut versions: HashMap<String, Version> = HashMap::new();

        for package in self.packages.iter() {
            let repo_version = package.get_repo_version().unwrap();

            versions.insert(package.name.clone(), repo_version);
        }

        let max_version = versions.values().max().unwrap();

        let bump_version = prompt_bump_version(max_version.clone());

        println!("max version: {}", max_version);
        println!("bump version: {}", bump_version);
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

fn main() {
    let releaser = Releaser::new();

    let args = Args::parse();

    match args.command {
        Command::Bump { version } => {
            releaser.bump(version);
        }
    }
}
