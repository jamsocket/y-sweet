use super::PackageManager;
use crate::package_manager::get_client;
use anyhow::{Context, Result};
use semver::Version;
use serde::Deserialize;
use std::{fs, path::Path};
use toml_edit::{value, DocumentMut};

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

    fn get_repo_version(&self, path: &Path) -> Result<Version> {
        let cargo_toml = fs::read_to_string(path.join("Cargo.toml"))?;
        let cargo_toml: CargoToml = toml::from_str(&cargo_toml)?;
        let version = Version::parse(&cargo_toml.package.version)?;
        Ok(version)
    }

    fn set_repo_version(&self, path: &Path, version: &Version) -> Result<()> {
        let cargo_toml = fs::read_to_string(path.join("Cargo.toml"))?;
        let mut doc = cargo_toml.parse::<DocumentMut>()?;
        doc["package"]["version"] = value(version.to_string());
        fs::write(path.join("Cargo.toml"), doc.to_string())?;
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct CargoToml {
    package: CargoPackage,
}

#[derive(Debug, Deserialize)]
struct CargoPackage {
    version: String,
}

#[derive(Debug, Deserialize)]
struct CratesResponse {
    #[serde(rename = "crate")]
    _crate: CratesCrate,
}

#[derive(Debug, Deserialize)]
struct CratesCrate {
    newest_version: Version,
    max_version: Version,
    max_stable_version: Version,
}
