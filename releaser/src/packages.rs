use crate::package_manager::{Package, PackageType};
use std::path::PathBuf;

pub struct PackageList {
    base_dir: PathBuf,
    packages: Vec<Package>,
}

impl PackageList {
    pub fn new() -> Self {
        let base_dir = get_root_dir();
        let packages = Vec::new();
        Self { base_dir, packages }
    }

    pub fn register_cargo_package(&mut self, name: &str, path: &str) {
        let path = self.base_dir.join(path);
        let package = Package {
            name: name.to_string(),
            path,
            package_type: PackageType::Cargo,
        };
        self.packages.push(package);
    }

    pub fn register_node_package(&mut self, name: &str, path: &str) {
        let path = self.base_dir.join(path);
        let package = Package {
            name: name.to_string(),
            path,
            package_type: PackageType::Node,
        };
        self.packages.push(package);
    }

    pub fn register_python_package(&mut self, name: &str, path: &str) {
        let path = self.base_dir.join(path);
        let package = Package {
            name: name.to_string(),
            path,
            package_type: PackageType::Python,
        };
        self.packages.push(package);
    }

    pub fn iter(&self) -> impl Iterator<Item = &Package> {
        self.packages.iter()
    }
}

fn get_root_dir() -> PathBuf {
    // crawl up the directory tree until we find a .git directory
    let mut path = PathBuf::from(".").canonicalize().unwrap();
    loop {
        if path.join(".git").exists() {
            return path;
        }
        if !path.pop() {
            panic!("Could not find root directory");
        }
    }
}
