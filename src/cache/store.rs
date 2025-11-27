// Cache store for reading and writing cached data.
// Handles JSON serialization, TTL checking, and filesystem operations.

use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::{Duration, SystemTime};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::error::Result;

/// Default TTL for mutable data (runners, active runs): 5 minutes.
pub const DEFAULT_TTL: Duration = Duration::from_secs(5 * 60);

/// Wrapper for cached data with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedData<T> {
    /// The cached data.
    pub data: T,
    /// When the data was cached.
    pub cached_at: DateTime<Utc>,
    /// Whether this data is immutable (completed runs, logs).
    pub immutable: bool,
}

impl<T> CachedData<T> {
    /// Create a new cached data entry.
    pub fn new(data: T, immutable: bool) -> Self {
        Self {
            data,
            cached_at: Utc::now(),
            immutable,
        }
    }

    /// Check if this cached data has expired based on TTL.
    pub fn is_expired(&self, ttl: Duration) -> bool {
        if self.immutable {
            return false;
        }

        let elapsed = Utc::now()
            .signed_duration_since(self.cached_at)
            .to_std()
            .unwrap_or(Duration::MAX);

        elapsed > ttl
    }

    /// Check if this cached data is still valid (not expired).
    pub fn is_valid(&self, ttl: Duration) -> bool {
        !self.is_expired(ttl)
    }
}

/// Read cached JSON data from a file.
pub fn read_cached<T: DeserializeOwned>(path: &Path) -> Result<Option<CachedData<T>>> {
    if !path.exists() {
        return Ok(None);
    }

    let contents = fs::read_to_string(path)?;
    let cached: CachedData<T> = serde_json::from_str(&contents)?;
    Ok(Some(cached))
}

/// Read cached JSON data, returning None if expired.
pub fn read_if_valid<T: DeserializeOwned>(path: &Path, ttl: Duration) -> Result<Option<T>> {
    match read_cached::<T>(path)? {
        Some(cached) if cached.is_valid(ttl) => Ok(Some(cached.data)),
        _ => Ok(None),
    }
}

/// Write data to cache as JSON.
pub fn write_cached<T: Serialize>(path: &Path, data: &T, immutable: bool) -> Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let cached = CachedData::new(data, immutable);
    let json = serde_json::to_string_pretty(&cached)?;

    // Write atomically via temp file
    let temp_path = path.with_extension("tmp");
    let mut file = fs::File::create(&temp_path)?;
    file.write_all(json.as_bytes())?;
    file.sync_all()?;
    fs::rename(&temp_path, path)?;

    Ok(())
}

/// Write raw text data to cache (for logs).
pub fn write_text(path: &Path, text: &str) -> Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write atomically via temp file
    let temp_path = path.with_extension("tmp");
    let mut file = fs::File::create(&temp_path)?;
    file.write_all(text.as_bytes())?;
    file.sync_all()?;
    fs::rename(&temp_path, path)?;

    Ok(())
}

/// Read raw text data from cache (for logs).
pub fn read_text(path: &Path) -> Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }

    let contents = fs::read_to_string(path)?;
    Ok(Some(contents))
}

/// Check if a cache file exists.
pub fn exists(path: &Path) -> bool {
    path.exists()
}

/// Get the modification time of a cache file.
pub fn modified_at(path: &Path) -> io::Result<SystemTime> {
    fs::metadata(path)?.modified()
}

/// Delete a cached file.
pub fn delete(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

/// Delete a cached directory and all contents.
pub fn delete_dir(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}

/// Invalidate cache for an owner (delete all cached data).
pub fn invalidate_owner(owner: &str) -> Result<()> {
    if let Some(dir) = super::paths::owner_dir(owner) {
        delete_dir(&dir)?;
    }
    Ok(())
}

/// Invalidate cache for a repository.
pub fn invalidate_repo(owner: &str, repo: &str) -> Result<()> {
    if let Some(dir) = super::paths::repo_dir(owner, repo) {
        delete_dir(&dir)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestData {
        name: String,
        value: i32,
    }

    #[test]
    fn test_write_and_read_cached() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.json");

        let data = TestData {
            name: "test".to_string(),
            value: 42,
        };

        write_cached(&path, &data, false).unwrap();

        let cached: Option<CachedData<TestData>> = read_cached(&path).unwrap();
        assert!(cached.is_some());

        let cached = cached.unwrap();
        assert_eq!(cached.data, data);
        assert!(!cached.immutable);
    }

    #[test]
    fn test_immutable_never_expires() {
        let data = CachedData::new("test", true);

        // Even with zero TTL, immutable data should not expire
        assert!(!data.is_expired(Duration::ZERO));
        assert!(data.is_valid(Duration::ZERO));
    }

    #[test]
    fn test_mutable_expires() {
        let mut data = CachedData::new("test", false);

        // Set cached_at to the past
        data.cached_at = Utc::now() - chrono::Duration::seconds(600);

        // Should be expired with 5 minute TTL
        assert!(data.is_expired(Duration::from_secs(300)));
        assert!(!data.is_valid(Duration::from_secs(300)));
    }

    #[test]
    fn test_write_and_read_text() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("log.txt");

        let text = "Line 1\nLine 2\nLine 3";

        write_text(&path, text).unwrap();

        let read = read_text(&path).unwrap();
        assert_eq!(read, Some(text.to_string()));
    }

    #[test]
    fn test_read_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("nonexistent.json");

        let cached: Option<CachedData<TestData>> = read_cached(&path).unwrap();
        assert!(cached.is_none());

        let text = read_text(&path).unwrap();
        assert!(text.is_none());
    }
}
