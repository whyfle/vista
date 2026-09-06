use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FlathubApp {
    pub app_id: String,
    pub name: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub version: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FlathubSearchResult {
    pub hits: Vec<FlathubHit>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FlathubHit {
    pub app_id: String,
    pub name: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub installs_last_month: Option<u64>,
    pub keywords: Option<Vec<String>>,
}

// For Flathub API v2: https://flathub.org/api/v2/search/{query}  and /apps/{app_id}
#[derive(Debug, Clone)]
pub struct FlathubProvider {
    pub api_base: String,
    pub user_agent: String,
}

impl FlathubProvider {
    pub fn new(api_base: String) -> Self {
        Self { api_base, user_agent: format!("vista/{}", env!("CARGO_PKG_VERSION")) }
    }

    pub fn from_config(cfg: &crate::config::Config) -> Self {
        Self::new(cfg.flathub.api_url.clone())
    }

    fn build_req(&self, url: &str) -> ureq::Request {
        ureq::get(url).set("User-Agent", &self.user_agent).set("Accept", "application/json")
    }

    pub fn search(&self, query: &str) -> anyhow::Result<Vec<FlathubHit>> {
        // Flathub API v2: POST {api_base}/search with JSON {"query": "..."}
        // Docs: https://flathub.org/api/v2/docs — GET /search/:query is gone,
        // correct is POST /search. Response: {"hits": [{"app_id":...,"name":...}]}
        let url = format!("{}/search", self.api_base.trim_end_matches('/'));
        let body = serde_json::json!({"query": query});
        let req = ureq::post(&url)
            .set("User-Agent", &self.user_agent)
            .set("Accept", "application/json")
            .set("Content-Type", "application/json");
        match req.send_json(body) {
            Ok(resp) => {
                if resp.status() >= 400 {
                    anyhow::bail!("Flathub search API error {}", resp.status());
                }
                let text = resp.into_string()?;
                if let Ok(parsed) = serde_json::from_str::<FlathubSearchResult>(&text) {
                    Ok(parsed.hits)
                } else if let Ok(parsed) = serde_json::from_str::<Vec<FlathubHit>>(&text) {
                    Ok(parsed)
                } else if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some(arr) = parsed.get("hits").and_then(|v| v.as_array()) {
                        let hits: Vec<FlathubHit> = serde_json::from_value(serde_json::Value::Array(arr.clone())).unwrap_or_default();
                        Ok(hits)
                    } else if let Some(arr) = parsed.as_array() {
                        let hits: Vec<FlathubHit> = serde_json::from_value(serde_json::Value::Array(arr.clone())).unwrap_or_default();
                        Ok(hits)
                    } else {
                        Ok(Vec::new())
                    }
                } else {
                    Ok(Vec::new())
                }
            },
            Err(e) => {
                // Fallback to local flatpak search (no network API dependency)
                if let Ok(local) = Self::search_local_flatpak(query) {
                    if !local.is_empty() {
                        let hits: Vec<FlathubHit> = local.into_iter().map(|app_id| FlathubHit {
                            app_id: app_id.clone(),
                            name: app_id.clone(),
                            summary: None,
                            description: None,
                            icon: None,
                            installs_last_month: None,
                            keywords: None,
                        }).collect();
                        return Ok(hits);
                    }
                }
                anyhow::bail!("Flathub search failed POST {}: {}", url, e);
            }
        }
    }

    pub fn get_app(&self, app_id: &str) -> anyhow::Result<FlathubApp> {
        // v2 canonical: GET /appstream/{app_id}
        let url = format!("{}/appstream/{}", self.api_base.trim_end_matches('/'), app_id);
        let req = self.build_req(&url);
        let resp = req.call().map_err(|e| anyhow::anyhow!("Flathub get app failed {}: {}", url, e))?;
        if resp.status() >= 400 {
            anyhow::bail!("Flathub app not found {}: {}", app_id, resp.status());
        }
        let app: FlathubApp = resp.into_json()?;
        Ok(app)
    }

    pub fn is_available(&self) -> bool {
        // Check if flatpak is installed? But API availability is independent
        true
    }

    // Helper for testing / alternative: search via summary API using flatpak's own search would need local flatpak
    pub fn search_local_flatpak(query: &str) -> anyhow::Result<Vec<String>> {
        // Try using `flatpak search --columns=application` if flatpak exists
        let output = std::process::Command::new("flatpak").args(&["search", "--columns=application", query]).output();
        match output {
            Ok(o) if o.status.success() => {
                let text = String::from_utf8_lossy(&o.stdout);
                let apps: Vec<String> = text.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect();
                Ok(apps)
            },
            _ => Ok(Vec::new()),
        }
    }
}

fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalFlatpakSearch {
    pub app_id: String,
    pub name: String,
    pub description: Option<String>,
}
