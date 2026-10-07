use std::path::{Path, PathBuf};

use async_trait::async_trait;

use crate::error::AppError;

#[async_trait]
#[allow(dead_code)]
pub trait StorageProvider: Send + Sync {
    async fn upload(&self, key: &str, bytes: &[u8]) -> Result<String, AppError>;
    async fn delete(&self, key: &str) -> Result<(), AppError>;
    async fn get_url(&self, key: &str) -> Result<String, AppError>;
}

#[allow(dead_code)]
pub struct LocalStorage {
    root: PathBuf,
}
impl LocalStorage {
    #[allow(dead_code)]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

#[async_trait]
impl StorageProvider for LocalStorage {
    async fn upload(&self, key: &str, bytes: &[u8]) -> Result<String, AppError> {
        validate_key(key)?;
        let path = self.root.join(key);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(anyhow::Error::from)?;
        }
        tokio::fs::write(&path, bytes)
            .await
            .map_err(anyhow::Error::from)?;
        self.get_url(key).await
    }
    async fn delete(&self, key: &str) -> Result<(), AppError> {
        validate_key(key)?;
        match tokio::fs::remove_file(self.root.join(key)).await {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(AppError::Internal(error.into())),
        }
    }
    async fn get_url(&self, key: &str) -> Result<String, AppError> {
        validate_key(key)?;
        Ok(format!("/uploads/{key}"))
    }
}

fn validate_key(key: &str) -> Result<(), AppError> {
    let path = Path::new(key);
    let safe = !key.is_empty()
        && !path.is_absolute()
        && !path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir));
    safe.then_some(())
        .ok_or_else(|| AppError::Validation("Invalid storage key".into()))
}

#[async_trait]
#[allow(dead_code)]
pub trait SearchProvider: Send + Sync {
    type Result: Send;
    async fn search(
        &self,
        organization_id: uuid::Uuid,
        query: &str,
        limit: u32,
    ) -> Result<Vec<Self::Result>, AppError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn storage_keys_cannot_escape_root() {
        assert!(validate_key("articles/file.pdf").is_ok());
        assert!(validate_key("../secret").is_err());
        assert!(validate_key("/etc/passwd").is_err());
    }
}
