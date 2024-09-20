use super::{PackageInfo, PackageManager};
use crate::package_manager::get_client;
use anyhow::{Context, Result};
use semver::Version;
use serde::Deserialize;
use serde_json::{Map, Value};
use std::{fs, path::Path, process::Command};

pub struct NodePackageManager;

impl PackageManager for NodePackageManager {
    fn get_published_version(&self, package: &str) -> Result<Version> {
        let client = get_client();

        let url = format!("https://registry.npmjs.org/{}", package);
        let response = client
            .get(url)
            .send()
            .context("Making request to npm registry")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to get public version for package {}",
                package
            ));
        }

        let version = response
            .json::<NpmResponse>()
            .context("Parsing response from npm registry")?
            .dist_tags
            .latest;

        Ok(Version::parse(&version)?)
    }

    fn get_package_info(&self, path: &Path) -> Result<PackageInfo> {
        let package_json = fs::read_to_string(path.join("package.json"))?;
        let package_json: PackageJson = serde_json::from_str(&package_json)?;
        let version = Version::parse(&package_json.version)?;
        let name = package_json.name;
        let private = package_json.private;
        Ok(PackageInfo {
            version,
            private,
            name,
        })
    }

    fn set_repo_version(&self, path: &Path, version: &Version) -> Result<()> {
        let package_json = fs::read_to_string(path.join("package.json"))?;
        let mut doc: Map<String, Value> = serde_json::from_str(&package_json)?;
        doc["version"] = Value::String(version.to_string());
        let mut pretty_json = serde_json::to_string_pretty(&doc)?;
        pretty_json.push('\n');

        fs::write(path.join("package.json"), &pretty_json)?;
        Ok(())
    }

    fn update_dependencies(
        &self,
        path: &Path,
        _deps: &[String],
        _version: &Version,
    ) -> Result<bool> {
        let package_file = path.join("package.json");
        let package_json = fs::read_to_string(&package_file)?;
        let mut updated: bool = false;

        let mut doc: Map<String, Value> = serde_json::from_str(&package_json)?;
        if let Some(deps) = doc.get_mut("dependencies") {
            for dep in _deps {
                if let Some(dep_version) = deps.get_mut(dep) {
                    *dep_version = Value::String(_version.to_string());
                    updated = true;
                }
            }
        }

        if updated {
            let mut pretty_json = serde_json::to_string_pretty(&doc)?;
            pretty_json.push('\n');
            fs::write(&package_file, &pretty_json)?;
        }

        Ok(updated)
    }

    fn update_lockfile(&self, path: &Path) -> Result<()> {
        let working_dir = path.parent().unwrap();
        let status = Command::new("npm")
            .arg("install")
            .current_dir(working_dir)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("Failed to update lockfile"));
        }
        Ok(())
    }

    fn publish(&self, path: &Path) -> Result<()> {
        let status = Command::new("npm")
            .arg("publish")
            .current_dir(&path)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("Failed to publish package"));
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct NpmResponse {
    #[serde(rename = "dist-tags")]
    dist_tags: DistTags,
}

#[derive(Debug, Deserialize)]
struct DistTags {
    latest: String,
}

#[derive(Debug, Deserialize)]
struct PackageJson {
    version: String,
    name: String,
    #[serde(default)]
    private: bool,
}
