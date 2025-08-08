use async_trait::async_trait;
use std::{
    fs::{create_dir_all, remove_file},
    path::PathBuf,
};
use y_sweet_core::store::{Result, Store, StoreError};

pub struct FileSystemStore {
    base_path: PathBuf,
}

impl FileSystemStore {
    pub fn new(base_path: PathBuf) -> std::result::Result<Self, std::io::Error> {
        create_dir_all(base_path.clone())?;
        Ok(Self { base_path })
    }
}

#[async_trait]
impl Store for FileSystemStore {
    async fn init(&self) -> Result<()> {
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let path = self.base_path.join(key);
        let contents = std::fs::read(path);
        match contents {
            Ok(contents) => Ok(Some(contents)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(StoreError::ConnectionError(e.to_string())),
        }
    }

    async fn set(&self, key: &str, value: Vec<u8>) -> Result<()> {
        let path = self.base_path.join(key);
        create_dir_all(path.parent().expect("Bad parent"))
            .map_err(|_| StoreError::NotAuthorized("Error creating directories".to_string()))?;
        std::fs::write(path, value)
            .map_err(|_| StoreError::NotAuthorized("Error writing file.".to_string()))?;
        Ok(())
    }

    async fn remove(&self, key: &str) -> Result<()> {
        let path = self.base_path.join(key);
        remove_file(path)
            .map_err(|_| StoreError::NotAuthorized("Error removing file.".to_string()))?;
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        let path = self.base_path.join(key);
        Ok(path.exists())
    }

    async fn generate_upload_presigned_url(&self, key: &str) -> Result<String> {
        // For local filesystem, return a dummy URL that indicates local storage
        // This is not a real upload URL, but serves as a placeholder for local development
        Ok(format!("file://localhost/{}", key))
    }

    async fn generate_download_presigned_url(&self, key: &str) -> Result<String> {
        // For local filesystem, return a dummy URL that indicates local storage
        // This is not a real download URL, but serves as a placeholder for local development
        Ok(format!("file://localhost/{}", key))
    }

    async fn list_objects(&self, prefix: &str) -> Result<Vec<String>> {
        // For local filesystem, list files in the directory
        let path = self.base_path.join(prefix);
        if !path.exists() {
            return Ok(Vec::new());
        }

        let mut objects = Vec::new();
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let file_name = entry.file_name();
                    if let Ok(name) = file_name.into_string() {
                        objects.push(name);
                    }
                }
            }
        }

        Ok(objects)
    }

    async fn copy_document(&self, source_doc_id: &str, destination_doc_id: &str) -> Result<()> {
        use std::fs;
        use std::io;

        let source_path = self.base_path.join(source_doc_id);
        let destination_path = self.base_path.join(destination_doc_id);

        // Remove destination directory if it exists (to ensure clean copy)
        if destination_path.exists() {
            fs::remove_dir_all(&destination_path).map_err(|e| {
                StoreError::ConnectionError(format!(
                    "Failed to remove existing destination directory: {}",
                    e
                ))
            })?;
        }

        // Create destination directory
        fs::create_dir_all(&destination_path).map_err(|e| {
            StoreError::ConnectionError(format!("Failed to create destination directory: {}", e))
        })?;

        // Copy all files and subdirectories recursively
        fn copy_recursive(src: &std::path::Path, dst: &std::path::Path) -> io::Result<()> {
            if src.is_file() {
                fs::copy(src, dst)?;
            } else if src.is_dir() {
                fs::create_dir_all(dst)?;
                for entry in fs::read_dir(src)? {
                    let entry = entry?;
                    let file_type = entry.file_type()?;
                    let src_path = entry.path();
                    let dst_path = dst.join(entry.file_name());

                    if file_type.is_dir() {
                        copy_recursive(&src_path, &dst_path)?;
                    } else {
                        fs::copy(&src_path, &dst_path)?;
                    }
                }
            }
            Ok(())
        }

        copy_recursive(&source_path, &destination_path)
            .map_err(|e| StoreError::ConnectionError(format!("Failed to copy document: {}", e)))?;

        Ok(())
    }
}
