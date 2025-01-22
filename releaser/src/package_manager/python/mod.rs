use super::{PackageInfo, PackageManager};
use crate::package_manager::get_client;
use anyhow::{Context, Result};
use semver::Version;
use serde::Deserialize;
use std::{fs, path::Path, process::Command};
use toml_edit::{value, DocumentMut};
use virtualenv::VirtualEnv;

pub struct PythonPackageManager;

mod virtualenv;

impl PackageManager for PythonPackageManager {
    fn get_published_version(&self, package: &str) -> Result<Version> {
        let client = get_client()?;

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

    fn get_package_info(&self, path: &Path) -> Result<super::PackageInfo> {
        // use pyproject.toml to get the version
        let pyproject_toml = fs::read_to_string(path.join("pyproject.toml"))?;
        let pyproject_toml: PyProjectToml = toml::from_str(&pyproject_toml)?;
        let version = Version::parse(&pyproject_toml.project.version)?;
        let name = pyproject_toml.project.name;
        // Ref: https://packaging.python.org/en/latest/guides/writing-pyproject-toml/#classifiers
        let private = pyproject_toml
            .project
            .classifiers
            .iter()
            .any(|c| c.starts_with("Private ::"));
        Ok(PackageInfo {
            version,
            private,
            name,
        })
    }

    fn set_repo_version(&self, path: &Path, version: &Version) -> Result<()> {
        let pyproject_toml = fs::read_to_string(path.join("pyproject.toml"))?;
        let mut doc = pyproject_toml.parse::<DocumentMut>()?;
        doc["project"]["version"] = value(version.to_string());
        fs::write(path.join("pyproject.toml"), doc.to_string())?;
        Ok(())
    }

    fn update_dependencies(
        &self,
        _path: &Path,
        _deps: &[String],
        _version: &Version,
    ) -> Result<bool> {
        if _deps.len() > 1 {
            return Err(anyhow::anyhow!("Python dependencies not supported"));
        }

        Ok(false)
    }

    fn update_lockfile(&self, _path: &Path) -> Result<()> {
        // no lockfile for python?
        Ok(())
    }

    fn publish(&self, path: &Path) -> Result<()> {
        let env = VirtualEnv::new(&["build", "twine"])?;

        let python = env.python();

        // clean dist directory
        let _ = fs::remove_dir_all(path.join("dist"));

        // Build the package
        let build_output = Command::new(python)
            .arg("-m")
            .arg("build")
            .current_dir(path)
            .output()
            .context("Failed to execute build command")?;

        if !build_output.status.success() {
            return Err(anyhow::anyhow!(
                "Build failed with status: {}.\nStderr: {}",
                build_output.status,
                String::from_utf8_lossy(&build_output.stderr)
            ));
        }

        // Upload the package
        let upload_output = Command::new(python)
            .arg("-m")
            .arg("twine")
            .arg("upload")
            .arg("dist/*")
            .current_dir(path)
            .output()?;

        if !upload_output.status.success() {
            return Err(anyhow::anyhow!(
                "Upload failed with status: {}.\nStdout: {}\nStderr: {}",
                upload_output.status,
                String::from_utf8_lossy(&upload_output.stdout),
                String::from_utf8_lossy(&upload_output.stderr)
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct PyProjectToml {
    project: Project,
}

#[derive(Debug, Deserialize)]
struct Project {
    version: String,
    name: String,
    classifiers: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct PyPIResponse {
    info: PyPIInfo,
}

#[derive(Debug, Deserialize)]
struct PyPIInfo {
    version: String,
}
