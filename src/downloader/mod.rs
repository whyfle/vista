use std::path::{Path, PathBuf};
use std::io::{self, Write};

pub struct Downloader {
    pub user_agent: String,
}

impl Downloader {
    pub fn new() -> Self {
        Self { user_agent: format!("vista/{}", env!("CARGO_PKG_VERSION")) }
    }

    pub fn download(&self, url: &str, dest: &Path) -> anyhow::Result<u64> {
        // Ensure parent dir exists
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let req = ureq::get(url)
            .set("User-Agent", &self.user_agent)
            .set("Accept", "*/*");

        // Follow redirects automatically via ureq
        let resp = req.call().map_err(|e| anyhow::anyhow!("Download failed for {}: {}", url, e))?;
        if resp.status() >= 400 {
            anyhow::bail!("Download failed {}: HTTP {}", url, resp.status());
        }

        let len = resp.header("Content-Length").and_then(|v| v.parse::<u64>().ok());
        let mut reader = resp.into_reader();
        let mut file = std::fs::File::create(dest)?;
        let bytes = io::copy(&mut reader, &mut file)?;
        // Optionally show progress? For now simple
        if let Some(expected) = len {
            if bytes != expected {
                eprintln!("Warning: downloaded {} bytes but expected {}", bytes, expected);
            }
        }
        Ok(bytes)
    }

    pub fn download_with_progress(&self, url: &str, dest: &Path) -> anyhow::Result<u64> {
        println!("  Downloading {} -> {:?}", url, dest);
        self.download(url, dest)
    }

    pub fn ensure_cached_or_download(&self, url: &str, filename: &str, cache: &crate::cache::Cache) -> anyhow::Result<PathBuf> {
        let dest = cache.download_path(filename);
        if cache.is_cached_download(filename) {
            println!("  Using cached download {:?}", dest);
            return Ok(dest);
        }
        self.download_with_progress(url, &dest)?;
        Ok(dest)
    }
}

impl Default for Downloader {
    fn default() -> Self { Self::new() }
}
