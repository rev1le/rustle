//! Classification results cache (Stage 3) on the embedded KV-DB `sled`

use std::io::Read;
use std::path::{Path, PathBuf};

use blake3::Hasher;

use crate::error::RustleError;
use crate::models::Classification;

/// Streaming read buffer for hashing
const HASH_BUF_SIZE: usize = 64 * 1024;

/// Classification cache on `sled`, keyed by content hash
pub struct ClassificationCache {
    db: sled::Db,
}

impl ClassificationCache {
    /// Open at an explicit directory, created if missing
    pub fn open(path: &Path) -> Result<Self, RustleError> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    /// Open at the OS cache dir, falling back to local `.rustle-cache`
    pub fn new() -> Result<Self, RustleError> {
        Self::open(&default_cache_path())
    }

    /// Streaming BLAKE3 hash; used as the cache key and for duplicate detection
    pub fn hash_file(path: &Path) -> Result<[u8; 32], RustleError> {
        let mut file = std::fs::File::open(path)?;
        let mut hasher = Hasher::new();
        let mut buf = vec![0u8; HASH_BUF_SIZE];
        loop {
            let n = file.read(&mut buf)?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        Ok(*hasher.finalize().as_bytes())
    }

    /// Get a cached classification by hash
    pub fn get(&self, hash: &[u8; 32]) -> Result<Option<Classification>, RustleError> {
        let value = self.db.get(hash)?;
        value
            .map(|bytes| serde_json::from_slice(&bytes).map_err(RustleError::from))
            .transpose()
    }

    /// Insert a classification. sled flushes async; call [`flush`](Self::flush)
    /// for a durable write
    pub fn put(&self, hash: &[u8; 32], classification: Classification) -> Result<(), RustleError> {
        let bytes = serde_json::to_vec(&classification)?;
        self.db.insert(hash, bytes)?;
        Ok(())
    }

    /// Force a durable flush to disk
    pub fn flush(&self) -> Result<(), RustleError> {
        self.db.flush()?;
        Ok(())
    }

    /// Cached entry count
    pub fn len(&self) -> Result<usize, RustleError> {
        Ok(self.db.len())
    }

    /// Whether empty
    pub fn is_empty(&self) -> Result<bool, RustleError> {
        Ok(self.db.is_empty())
    }
}

/// Default cache location
fn default_cache_path() -> PathBuf {
    dirs::cache_dir()
        .map(|d| d.join("rustle").join("classifications"))
        .unwrap_or_else(|| PathBuf::from(".rustle-cache"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_file_is_stable_for_same_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        std::fs::write(&path, b"hello world").unwrap();

        let h1 = ClassificationCache::hash_file(&path).unwrap();
        let h2 = ClassificationCache::hash_file(&path).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash_file_differs_for_different_content() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.txt");
        let b = dir.path().join("b.txt");
        std::fs::write(&a, b"hello").unwrap();
        std::fs::write(&b, b"world").unwrap();

        assert_ne!(
            ClassificationCache::hash_file(&a).unwrap(),
            ClassificationCache::hash_file(&b).unwrap(),
        );
    }

    #[test]
    fn put_then_get_round_trips_classification() {
        let dir = tempfile::tempdir().unwrap();
        let cache = ClassificationCache::open(dir.path()).unwrap();
        let hash = [7u8; 32];

        cache.put(&hash, Classification::Trash).unwrap();

        assert_eq!(cache.get(&hash).unwrap(), Some(Classification::Trash));
    }

    #[test]
    fn put_then_get_preserves_uncertain_verdict() {
        let dir = tempfile::tempdir().unwrap();
        let cache = ClassificationCache::open(dir.path()).unwrap();
        let hash = [1u8; 32];

        cache.put(&hash, Classification::Uncertain).unwrap();

        assert_eq!(cache.get(&hash).unwrap(), Some(Classification::Uncertain));
    }

    #[test]
    fn get_returns_none_for_unknown_hash() {
        let dir = tempfile::tempdir().unwrap();
        let cache = ClassificationCache::open(dir.path()).unwrap();

        assert_eq!(cache.get(&[0u8; 32]).unwrap(), None);
    }

    #[test]
    fn put_overwrites_previous_value_for_same_hash() {
        let dir = tempfile::tempdir().unwrap();
        let cache = ClassificationCache::open(dir.path()).unwrap();
        let hash = [9u8; 32];

        cache.put(&hash, Classification::Keep).unwrap();
        cache.put(&hash, Classification::Trash).unwrap();

        assert_eq!(cache.get(&hash).unwrap(), Some(Classification::Trash));
    }

    #[test]
    fn cache_persists_across_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let hash = [3u8; 32];

        ClassificationCache::open(dir.path())
            .unwrap()
            .put(&hash, Classification::Keep)
            .unwrap();

        // reopen the same DB — data must survive
        let reopened = ClassificationCache::open(dir.path()).unwrap();
        assert_eq!(reopened.get(&hash).unwrap(), Some(Classification::Keep));
    }

    #[test]
    fn len_and_is_empty_reflect_entries() {
        let dir = tempfile::tempdir().unwrap();
        let cache = ClassificationCache::open(dir.path()).unwrap();

        assert!(cache.is_empty().unwrap());
        assert_eq!(cache.len().unwrap(), 0);

        cache.put(&[1u8; 32], Classification::Keep).unwrap();
        cache.put(&[2u8; 32], Classification::Trash).unwrap();

        assert_eq!(cache.len().unwrap(), 2);
        assert!(!cache.is_empty().unwrap());
    }
}
