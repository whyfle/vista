use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use serde::{Deserialize, Serialize};

pub struct Cache {
    pub dir: PathBuf,
    pub enabled: bool,
    pub ttl: Duration,
}

impl Cache {
    pub fn new(dir: PathBuf, enabled: bool, ttl_seconds: u64) -> Self {
        Self { dir, enabled, ttl: Duration::from_secs(ttl_seconds) }
    }

    pub fn from_config(cfg: &crate::config::Config) -> Self {
        let dir = cfg.cache_dir();
        Self::new(dir, cfg.cache.enabled, cfg.cache.ttl_seconds)
    }

    pub fn ensure_dir(&self) -> anyhow::Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        std::fs::create_dir_all(self.dir.join("metadata"))?;
        std::fs::create_dir_all(self.dir.join("downloads"))?;
        Ok(())
    }

    fn sanitize_key(key: &str) -> String {
        key.replace('/', "_").replace(':', "_").replace('?', "_").replace('&', "_")
    }

    pub fn metadata_path(&self, key: &str) -> PathBuf {
        self.dir.join("metadata").join(Self::sanitize_key(key) + ".json")
    }

    pub fn download_path(&self, filename: &str) -> PathBuf {
        self.dir.join("downloads").join(filename)
    }

    pub fn get_metadata<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        if !self.enabled { return None; }
        let path = self.metadata_path(key);
        if !path.exists() { return None; }
        // Check TTL
        if let Ok(meta) = std::fs::metadata(&path) {
            if let Ok(modified) = meta.modified() {
                if let Ok(elapsed) = SystemTime::now().duration_since(modified) {
                    if elapsed > self.ttl { return None; }
                }
            }
        }
        let content = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn set_metadata<T: Serialize>(&self, key: &str, value: &T) -> anyhow::Result<()> {
        if !self.enabled { return Ok(()); }
        self.ensure_dir()?;
        let path = self.metadata_path(key);
        let content = serde_json::to_string_pretty(value)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn is_cached_download(&self, filename: &str) -> bool {
        self.download_path(filename).exists()
    }

    pub fn clean(&self) -> anyhow::Result<()> {
        if self.dir.exists() {
            std::fs::remove_dir_all(&self.dir)?;
            self.ensure_dir()?;
        }
        Ok(())
    }

    pub fn clean_metadata(&self) -> anyhow::Result<()> {
        let meta_dir = self.dir.join("metadata");
        if meta_dir.exists() {
            std::fs::remove_dir_all(&meta_dir)?;
            std::fs::create_dir_all(meta_dir)?;
        }
        Ok(())
    }

    pub fn stats(&self) -> CacheStats {
        let mut stats = CacheStats::default();
        if let Ok(entries) = std::fs::read_dir(self.dir.join("metadata")) {
            for e in entries.flatten() {
                stats.metadata_files += 1;
                if let Ok(m) = e.metadata() { stats.metadata_size += m.len(); }
            }
        }
        if let Ok(entries) = std::fs::read_dir(self.dir.join("downloads")) {
            for e in entries.flatten() {
                stats.download_files += 1;
                if let Ok(m) = e.metadata() { stats.download_size += m.len(); }
            }
        }
        stats
    }
}

#[derive(Debug, Default)]
pub struct CacheStats {
    pub metadata_files: usize,
    pub metadata_size: u64,
    pub download_files: usize,
    pub download_size: u64,
}
