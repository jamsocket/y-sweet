use anyhow::{Context, Result};
use rand::Rng;
use std::{
    path::{Path, PathBuf},
    process::Command,
};

fn random_dir() -> PathBuf {
    let base = std::env::temp_dir();
    let mut rng = rand::thread_rng();
    let random_string = rng.gen::<u64>().to_string();
    base.join(random_string)
}

/// A virtual environment created from a static set of dependencies.
/// This is used to create a temporary build environment for Python packages.
/// When the VirtualEnv is dropped, the environment is destroyed.
pub struct VirtualEnv {
    path: PathBuf,
    python: PathBuf,
}

impl VirtualEnv {
    pub fn new(dependencies: &[&str]) -> Result<Self> {
        let path = random_dir();

        // Create the virtual environment
        Command::new("python3")
            .arg("-m")
            .arg("venv")
            .arg(&path)
            .output()
            .context("Creating virtualenv")?;

        let pip = path.join("bin").join("pip");
        let python = path.join("bin").join("python");

        // Install dependencies
        for dependency in dependencies {
            println!("Installing dependency: {}", dependency);
            Command::new(&pip)
                .arg("install")
                .arg(dependency)
                .output()
                .context("Installing dependencies")?;
        }

        println!("Virtual environment created at: {}", path.display());

        Ok(Self { path, python })
    }

    /// Get the path to the Python executable for this virtual environment.
    pub fn python(&self) -> &Path {
        &self.python
    }
}

impl Drop for VirtualEnv {
    fn drop(&mut self) {
        println!("Removing virtual environment at: {}", self.path.display());
        std::fs::remove_dir_all(self.path.clone()).ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_virtualenv() {
        let virtualenv = VirtualEnv::new(&["requests"]).unwrap();
        assert!(virtualenv.python().exists());
    }
}
