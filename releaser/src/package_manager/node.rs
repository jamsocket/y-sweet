use super::PackageManager;
use crate::package_manager::get_client;
use anyhow::{Context, Result};
use semver::Version;
use serde::Deserialize;
use serde_json::{Map, Value};
use std::{fs, path::Path};

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

    fn get_repo_version(&self, path: &Path) -> Result<Version> {
        let package_json = fs::read_to_string(path.join("package.json"))?;
        let package_json: PackageJson = serde_json::from_str(&package_json)?;
        let version = Version::parse(&package_json.version)?;
        Ok(version)
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

    fn update_dependencies(&self, _deps: &[String], _version: &Version) -> Result<bool> {
        let package_json = fs::read_to_string("package.json")?;
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
            fs::write("package.json", &pretty_json)?;
        }

        Ok(updated)
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
}
