use anyhow::Result;
use async_trait::async_trait;
use std::{
    fs::{create_dir_all, remove_file},
    path::PathBuf,
};
use y_sweet_core::store::Store;

pub struct FileSystemStore {
    base_path: PathBuf,
}

impl FileSystemStore {
    pub fn new(base_path: PathBuf) -> Result<Self, std::io::Error> {
        create_dir_all(base_path.clone())?;
        Ok(Self { base_path })
    }
}

#[async_trait]
impl Store for FileSystemStore {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let path = self.base_path.join(key);
        let contents = std::fs::read(path);
        match contents {
            Ok(contents) => Ok(Some(contents)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    async fn set(&self, key: &str, value: Vec<u8>) -> Result<()> {
        let path = self.base_path.join(key);
        create_dir_all(path.parent().expect("Bad parent"))?;
        std::fs::write(path, value)?;
        Ok(())
    }

    async fn remove(&self, key: &str) -> Result<()> {
        let path = self.base_path.join(key);
        remove_file(path)?;
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        let path = self.base_path.join(key);
        Ok(path.exists())
    }
}
