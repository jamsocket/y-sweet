use anyhow::Result;
use semver::Version;
use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

const APP_USER_AGENT: &str = "YSweet releaser";

pub mod cargo;
pub mod node;
pub mod python;

/// This trait captures the common functionality across package systems, both for accessing
/// local package files and for interacting with the remote package registries.
pub trait PackageManager {
    /// Get the latest published version of a package from the registry.
    fn get_published_version(&self, package: &str) -> Result<Version>;

    /// Get information about a package from a local path. Does not talk to the remote
    /// registry, and can be used for non-published packages.
    fn get_package_info(&self, path: &Path) -> Result<PackageInfo>;

    /// Set the version of a package locally in the repository.
    fn set_repo_version(&self, path: &Path, version: &Version) -> Result<()>;

    /// Update the dependencies of a package to the specified version.
    /// Returns Ok(true) if any dependencies were updated.
    fn update_dependencies(&self, path: &Path, deps: &[String], version: &Version) -> Result<bool>;

    /// Update the lockfile for a package, usually by running a build command.
    fn update_lockfile(&self, path: &Path) -> Result<()>;

    /// Publish a package to the remote registry. The underlying registry command may prompt
    /// for user credentials.
    fn publish(&self, path: &Path) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct PackageInfo {
    #[allow(unused)]
    pub name: String,
    pub version: Version,
    pub private: bool,
}

#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub enum PackageType {
    Cargo,
    Node,
    Python,
}

impl Display for PackageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackageType::Cargo => write!(f, "Cargo"),
            PackageType::Node => write!(f, "Node"),
            PackageType::Python => write!(f, "Python"),
        }
    }
}

impl PackageType {
    pub fn get_package_manager(&self) -> Box<dyn PackageManager> {
        match self {
            PackageType::Cargo => Box::new(cargo::CargoPackageManager),
            PackageType::Node => Box::new(node::NodePackageManager),
            PackageType::Python => Box::new(python::PythonPackageManager),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Package {
    pub name: String,
    pub path: PathBuf,
    pub package_type: PackageType,
}

impl Package {
    pub fn get_published_version(&self) -> Result<Version> {
        self.package_type
            .get_package_manager()
            .get_published_version(&self.name)
    }

    pub fn get_package_info(&self) -> Result<PackageInfo> {
        self.package_type
            .get_package_manager()
            .get_package_info(&self.path)
    }

    pub fn set_repo_version(&self, version: &Version) -> Result<()> {
        self.package_type
            .get_package_manager()
            .set_repo_version(&self.path, version)
    }

    pub fn publish(&self) -> Result<()> {
        self.package_type.get_package_manager().publish(&self.path)
    }
}

pub fn get_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .user_agent(APP_USER_AGENT)
        .build()
        .unwrap()
}
