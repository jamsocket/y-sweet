use std::{path::PathBuf, fs::remove_file, error::Error};
use async_trait::async_trait;

#[async_trait]
pub trait Store: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, Box<dyn Error>>;
    async fn set(&mut self, key: &str, value: Vec<u8>) -> Result<(), Box<dyn Error>>;
    async fn remove(&mut self, key: &str) -> Result<(), Box<dyn Error>>;
}

pub struct FileSystemStore {
    base_path: PathBuf,
}

impl FileSystemStore {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }
}

#[async_trait]
impl Store for FileSystemStore {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
        let path = self.base_path.join(key);
        let contents = std::fs::read(path);
        match contents {
            Ok(contents) => Ok(Some(contents)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    async fn set(&mut self, key: &str, value: Vec<u8>) -> Result<(), Box<dyn Error>> {
        let path = self.base_path.join(key);
        std::fs::write(path, value)?;
        Ok(())
    }

    async fn remove(&mut self, key: &str) -> Result<(), Box<dyn Error>> {
        let path = self.base_path.join(key);
        remove_file(path)?;
        Ok(())
    }
}
