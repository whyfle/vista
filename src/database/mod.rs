use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InstallSource {
    Github,
    Flathub,
    Native,
    Manual,
}

impl std::fmt::Display for InstallSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstallSource::Github => write!(f, "GitHub"),
            InstallSource::Flathub => write!(f, "Flathub"),
            InstallSource::Native => write!(f, "native"),
            InstallSource::Manual => write!(f, "manual"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPackage {
    pub name: String,
    pub version: String,
    pub source: InstallSource,
    pub repository: Option<String>, // e.g., user@repo or flathub app id
    pub installation_method: String, // e.g., rpm, deb, flatpak
    pub package_identifier: String, // filename or app id
    pub architecture: String,
    pub checksum: Option<String>,
    pub installed_at: DateTime<Utc>,
    pub install_path: Option<String>,
    pub download_url: Option<String>,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Database {
    pub packages: HashMap<String, InstalledPackage>,
    pub version: u32,
}

impl Database {
    pub fn new() -> Self { Self { packages: HashMap::new(), version: 1 } }

    pub fn db_path() -> Option<PathBuf> {
        dirs::data_dir().map(|p| p.join("vista").join("database.json"))
            .or_else(|| dirs::config_dir().map(|p| p.join("vista").join("database.json")))
    }

    pub fn db_dir() -> Option<PathBuf> {
        Self::db_path().and_then(|p| p.parent().map(|d| d.to_path_buf()))
    }

    pub fn load() -> Self {
        if let Some(path) = Self::db_path() {
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(db) = serde_json::from_str::<Database>(&content) {
                        return db;
                    }
                    // Try legacy format: Vec<InstalledPackage>
                    if let Ok(vec) = serde_json::from_str::<Vec<InstalledPackage>>(&content) {
                        let mut map = HashMap::new();
                        for pkg in vec { map.insert(pkg.name.clone(), pkg); }
                        return Database { packages: map, version: 1 };
                    }
                }
            }
        }
        Database::new()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        if let Some(dir) = Self::db_dir() {
            std::fs::create_dir_all(&dir)?;
        }
        if let Some(path) = Self::db_path() {
            let content = serde_json::to_string_pretty(self)?;
            std::fs::write(path, content)?;
        }
        Ok(())
    }

    pub fn add(&mut self, pkg: InstalledPackage) {
        self.packages.insert(pkg.name.clone(), pkg);
    }

    pub fn remove(&mut self, name: &str) -> Option<InstalledPackage> {
        self.packages.remove(name)
    }

    pub fn get(&self, name: &str) -> Option<&InstalledPackage> {
        self.packages.get(name)
    }

    pub fn list(&self) -> Vec<&InstalledPackage> {
        let mut v: Vec<&InstalledPackage> = self.packages.values().collect();
        v.sort_by(|a,b| a.name.cmp(&b.name));
        v
    }

    pub fn contains(&self, name: &str) -> bool {
        self.packages.contains_key(name)
    }

    pub fn count(&self) -> usize {
        self.packages.len()
    }
}
