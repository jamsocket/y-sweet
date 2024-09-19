use super::PackageManager;
use crate::package_manager::get_client;
use anyhow::{Context, Result};
use semver::Version;
use serde::Deserialize;
use std::{fs, path::Path};

pub struct PythonPackageManager;

impl PackageManager for PythonPackageManager {
    fn get_public_version(&self, package: &str) -> Result<Version> {
        let client = get_client();

        let url = format!("https://pypi.org/pypi/{}/json", package);
        let response = client.get(url).send().context("Making request to PyPI")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to get public version for package {}",
                package
            ));
        }

        let version = response
            .json::<PyPIResponse>()
            .context("Parsing response from PyPI")?
            .info
            .version;

        Ok(Version::parse(&version)?)
    }

    fn get_repo_version(&self, path: &Path) -> Result<Version> {
        // use pyproject.toml to get the version
        let pyproject_toml = fs::read_to_string(path.join("pyproject.toml"))?;
        let pyproject_toml: PyProjectToml = toml::from_str(&pyproject_toml)?;
        let version = Version::parse(&pyproject_toml.project.version)?;
        Ok(version)
    }

    fn set_repo_version(&self, path: &Path, version: &Version) -> Result<()> {
        panic!();
    }
}

#[derive(Debug, Deserialize)]
struct PyProjectToml {
    project: Project,
}

#[derive(Debug, Deserialize)]
struct Project {
    version: String,
}

#[derive(Debug, Deserialize)]
struct PyPIResponse {
    info: PyPIInfo,
}

#[derive(Debug, Deserialize)]
struct PyPIInfo {
    version: String,
}
