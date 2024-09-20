use super::{PackageInfo, PackageManager};
use crate::package_manager::get_client;
use anyhow::{Context, Result};
use semver::Version;
use serde::Deserialize;
use std::{fs, path::Path, process::Command};
use toml_edit::{value, DocumentMut, Table};

pub struct CargoPackageManager;

impl PackageManager for CargoPackageManager {
    fn get_published_version(&self, package: &str) -> Result<Version> {
        let client = get_client();

        // using reqwest and the crates.io API, get the latest version of the package
        // and parse it into a Version

        let url = format!("https://crates.io/api/v1/crates/{}", package);
        let response = client
            .get(url)
            .send()
            .context("Making request to crates.io")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to get public version for package {}",
                package
            ));
        }

        let version = response
            .json::<CratesResponse>()
            .context("Parsing response from crates.io")?
            ._crate
            .max_version;
        Ok(version)
    }

    fn get_package_info(&self, path: &Path) -> Result<PackageInfo> {
        let cargo_toml = fs::read_to_string(path.join("Cargo.toml"))?;
        let cargo_toml: CargoToml = toml::from_str(&cargo_toml)?;
        let version = Version::parse(&cargo_toml.package.version)?;
        let private = !cargo_toml.package.publish;
        let name = cargo_toml.package.name;
        Ok(PackageInfo {
            version,
            private,
            name,
        })
    }

    fn set_repo_version(&self, path: &Path, version: &Version) -> Result<()> {
        let cargo_toml = fs::read_to_string(path.join("Cargo.toml"))?;
        let mut doc = cargo_toml.parse::<DocumentMut>()?;
        doc["package"]["version"] = value(version.to_string());
        fs::write(path.join("Cargo.toml"), doc.to_string())?;
        Ok(())
    }

    fn update_dependencies(&self, path: &Path, deps: &[String], version: &Version) -> Result<bool> {
        // update the dependencies in Cargo.toml to the specified version
        // only update dependencies that the package actually has
        // use toml_edit to update the dependencies
        let mut updated = false;

        let cargo_file = path.join("Cargo.toml");
        let cargo_toml = fs::read_to_string(&cargo_file)?;
        let mut doc = cargo_toml.parse::<DocumentMut>()?;

        updated |= update_dep_table(doc["dependencies"].as_table_mut().unwrap(), deps, version);
        updated |= update_dep_table(
            doc["dev-dependencies"].as_table_mut().unwrap(),
            deps,
            version,
        );

        if updated {
            fs::write(&cargo_file, doc.to_string())?;
        }

        Ok(updated)
    }

    fn update_lockfile(&self, path: &Path) -> Result<()> {
        // run cargo check to update the lockfile
        let working_dir = path.parent().unwrap();
        let status = Command::new("cargo")
            .arg("check")
            .current_dir(working_dir)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("Failed to update lockfile"));
        }
        Ok(())
    }

    fn publish(&self, path: &Path) -> Result<()> {
        let status = Command::new("cargo")
            .arg("publish")
            .current_dir(path)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("Failed to publish package"));
        }
        Ok(())
    }
}

fn update_dep_table(table: &mut Table, deps: &[String], version: &Version) -> bool {
    let mut updated = false;
    for dep in deps {
        if let Some(dep_version) = table.get_mut(dep) {
            // if the dep_version is an object, we want to edit the version field of it
            // otherwise, we want to replace the value with a string

            if let Some(version) = dep_version.as_str() {
                *dep_version = value(version.to_string());
            } else if let Some(table) = dep_version.as_table_like_mut() {
                table.insert("version", value(version.to_string()));
            } else {
                continue;
            };

            updated = true;
        }
    }
    updated
}

#[derive(Debug, Deserialize)]
struct CargoToml {
    package: CargoPackage,
}

#[derive(Debug, Deserialize)]
struct CargoPackage {
    version: String,
    name: String,
    #[serde(default = "default_publish")]
    publish: bool,
}

fn default_publish() -> bool {
    true
}

#[derive(Debug, Deserialize)]
struct CratesResponse {
    #[serde(rename = "crate")]
    _crate: CratesCrate,
}

#[derive(Debug, Deserialize)]
struct CratesCrate {
    #[allow(unused)]
    newest_version: Version,
    max_version: Version,
    #[allow(unused)]
    max_stable_version: Version,
}
