use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub resolver: ResolverConfig,
    pub security: SecurityConfig,
    pub cache: CacheConfig,
    pub github: GithubConfig,
    pub flathub: FlathubConfig,
    pub general: GeneralConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolverConfig {
    pub prefer_native: bool,
    pub allow_flatpak: bool,
    pub allow_snap: bool,
    pub prefer_stable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub verify_checksums: bool,
    pub require_confirmation: bool,
    pub allow_untrusted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub enabled: bool,
    pub ttl_seconds: u64,
    pub directory: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubConfig {
    pub api_token: Option<String>,
    pub api_url: String,
    pub rate_limit_warn: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlathubConfig {
    pub enabled: bool,
    pub api_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub auto_confirm: bool,
    pub architecture: Option<String>,
    pub verbose: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            resolver: ResolverConfig {
                prefer_native: true,
                allow_flatpak: true,
                allow_snap: true,
                prefer_stable: true,
            },
            security: SecurityConfig {
                verify_checksums: true,
                require_confirmation: true,
                allow_untrusted: false,
            },
            cache: CacheConfig {
                enabled: true,
                ttl_seconds: 3600,
                directory: None,
            },
            github: GithubConfig {
                api_token: None,
                api_url: "https://api.github.com".to_string(),
                rate_limit_warn: true,
            },
            flathub: FlathubConfig {
                enabled: true,
                api_url: "https://flathub.org/api/v2".to_string(),
            },
            general: GeneralConfig {
                auto_confirm: false,
                architecture: None,
                verbose: false,
            },
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let mut cfg = Config::default();

        // Check env vars
        if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            cfg.github.api_token = Some(token);
        } else if let Ok(token) = std::env::var("VISTA_GITHUB_TOKEN") {
            cfg.github.api_token = Some(token);
        }

        // Try to load from file
        if let Some(path) = Self::config_path() {
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(parsed) = toml::from_str::<Config>(&content) {
                        cfg = parsed;
                        if let Ok(token) = std::env::var("GITHUB_TOKEN") {
                            cfg.github.api_token = Some(token);
                        } else if let Ok(token) = std::env::var("VISTA_GITHUB_TOKEN") {
                            cfg.github.api_token = Some(token);
                        }
                    } else {
                        eprintln!("Warning: failed to parse config at {:?}, using defaults", path);
                    }
                }
            }
        }

        cfg
    }

    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("vista").join("config.toml"))
    }

    pub fn config_dir() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("vista"))
    }

    pub fn ensure_config_dir() -> anyhow::Result<PathBuf> {
        let dir = Self::config_dir().ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    pub fn cache_dir(&self) -> PathBuf {
        if let Some(dir) = &self.cache.directory {
            PathBuf::from(dir)
        } else {
            dirs::cache_dir().map(|p| p.join("vista")).unwrap_or_else(|| PathBuf::from("/tmp/vista-cache"))
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let dir = Self::ensure_config_dir()?;
        let path = dir.join("config.toml");
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

pub fn init_default_config_if_missing() -> anyhow::Result<()> {
    if let Some(path) = Config::config_path() {
        if !path.exists() {
            let cfg = Config::default();
            cfg.save()?;
            println!("Created default config at {:?}", path);
        }
    }
    Ok(())
}
