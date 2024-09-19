use anyhow::Result;
use semver::Version;
use std::path::{Path, PathBuf};

const APP_USER_AGENT: &str = "YSweet releaser";

pub mod cargo;
pub mod node;
pub mod python;

trait PackageManager {
    fn get_public_version(&self, package: &str) -> Result<Version>;

    fn get_repo_version(&self, path: &Path) -> Result<Version>;
}

pub enum PackageType {
    Cargo,
    Node,
    Python,
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

pub struct Package {
    pub name: String,
    pub path: PathBuf,
    pub package_type: PackageType,
}

impl Package {
    pub fn get_public_version(&self) -> Result<Version> {
        self.package_type
            .get_package_manager()
            .get_public_version(&self.name)
    }

    pub fn get_repo_version(&self) -> Result<Version> {
        self.package_type
            .get_package_manager()
            .get_repo_version(&self.path)
    }
}

pub fn get_client() -> reqwest::blocking::Client {
    let client = reqwest::blocking::Client::builder()
        .user_agent(APP_USER_AGENT)
        .build()
        .unwrap();
    client
}
