use serde::{Deserialize, Serialize};
use crate::packages::PackageAsset;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GithubRepo {
    pub full_name: String,
    pub name: String,
    pub owner: Owner,
    pub description: Option<String>,
    pub stargazers_count: Option<u64>,
    pub html_url: String,
    pub default_branch: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Owner {
    pub login: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GithubRelease {
    pub tag_name: String,
    pub name: Option<String>,
    pub body: Option<String>,
    pub prerelease: bool,
    pub draft: bool,
    pub created_at: Option<String>,
    pub published_at: Option<String>,
    pub html_url: String,
    pub assets: Vec<GithubAsset>,
    pub target_commitish: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GithubAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
    pub download_count: Option<u64>,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GithubProvider {
    pub api_base: String,
    pub token: Option<String>,
    pub user_agent: String,
}

impl GithubProvider {
    pub fn new(api_base: String, token: Option<String>) -> Self {
        Self { api_base, token, user_agent: format!("vista/{}", env!("CARGO_PKG_VERSION")) }
    }

    pub fn from_config(cfg: &crate::config::Config) -> Self {
        Self::new(cfg.github.api_url.clone(), cfg.github.api_token.clone())
    }

    fn build_request(&self, url: &str) -> Result<ureq::Request, anyhow::Error> {
        let req = ureq::get(url)
            .set("User-Agent", &self.user_agent)
            .set("Accept", "application/vnd.github.v3+json");
        let req = if let Some(token) = &self.token {
            // GitHub API supports Bearer and token prefix
            if token.starts_with("ghp_") || token.starts_with("github_pat_") {
                req.set("Authorization", &format!("Bearer {}", token))
            } else {
                req.set("Authorization", &format!("token {}", token))
            }
        } else { req };
        Ok(req)
    }

    fn get_json<T: for<'de> Deserialize<'de>>(&self, url: &str) -> anyhow::Result<T> {
        let req = self.build_request(url)?;
        let resp = req.call().map_err(|e| anyhow::anyhow!("GitHub request failed for {}: {}", url, e))?;
        if resp.status() == 403 || resp.status() == 429 {
            anyhow::bail!("GitHub API rate limited (status {}). Consider setting GITHUB_TOKEN.", resp.status());
        }
        if resp.status() >= 400 {
            anyhow::bail!("GitHub API error {} for {}", resp.status(), url);
        }
        let data: T = resp.into_json()?;
        Ok(data)
    }

    pub fn parse_repo_spec(spec: &str) -> Option<(String, String)> {
        // Support: user@repo, user/repo, github.com/user/repo, https://github.com/user/repo
        let spec = spec.trim();
        if spec.contains('@') && !spec.contains('/') && !spec.contains(':') {
            // user@repo
            let parts: Vec<&str> = spec.split('@').collect();
            if parts.len() == 2 {
                return Some((parts[0].to_string(), parts[1].to_string()));
            }
        }
        if spec.contains("github.com") {
            // Extract user/repo from URL
            let url = spec.trim_start_matches("https://").trim_start_matches("http://").trim_start_matches("github.com/");
            let url = url.trim_start_matches('/').trim_end_matches('/').trim_end_matches(".git");
            let parts: Vec<&str> = url.split('/').collect();
            if parts.len() >= 2 {
                return Some((parts[parts.len()-2].to_string(), parts[parts.len()-1].to_string()));
            }
        }
        if spec.contains('/') {
            let parts: Vec<&str> = spec.split('/').collect();
            if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
                // Ensure not containing spaces or weird
                if !parts[0].contains(' ') && !parts[1].contains(' ') {
                    return Some((parts[0].to_string(), parts[1].to_string()));
                }
            }
        }
        None
    }

    pub fn repo_api_url(&self, owner: &str, repo: &str) -> String {
        format!("{}/repos/{}/{}", self.api_base.trim_end_matches('/'), owner, repo)
    }

    pub fn releases_api_url(&self, owner: &str, repo: &str) -> String {
        format!("{}/repos/{}/{}/releases", self.api_base.trim_end_matches('/'), owner, repo)
    }

    pub fn latest_release_api_url(&self, owner: &str, repo: &str) -> String {
        format!("{}/repos/{}/{}/releases/latest", self.api_base.trim_end_matches('/'), owner, repo)
    }

    pub fn get_repo(&self, owner: &str, repo: &str) -> anyhow::Result<GithubRepo> {
        let url = self.repo_api_url(owner, repo);
        self.get_json(&url)
    }

    pub fn get_latest_release(&self, owner: &str, repo: &str) -> anyhow::Result<GithubRelease> {
        let url = self.latest_release_api_url(owner, repo);
        self.get_json(&url)
    }

    pub fn get_releases(&self, owner: &str, repo: &str) -> anyhow::Result<Vec<GithubRelease>> {
        let url = self.releases_api_url(owner, repo);
        self.get_json(&url)
    }

    pub fn get_latest_stable_release(&self, owner: &str, repo: &str) -> anyhow::Result<GithubRelease> {
        // Try latest first, if prerelease/draft fallback to list
        match self.get_latest_release(owner, repo) {
            Ok(rel) if !rel.prerelease && !rel.draft => Ok(rel),
            _ => {
                let releases = self.get_releases(owner, repo)?;
                // Find latest stable
                let stable = releases.into_iter().find(|r| !r.prerelease && !r.draft);
                stable.ok_or_else(|| anyhow::anyhow!("No stable release found for {}/{}", owner, repo))
            }
        }
    }

    pub fn search_repos(&self, query: &str, per_page: u8) -> anyhow::Result<SearchResult> {
        let url = format!("{}/search/repositories?q={}&per_page={}&sort=stars&order=desc", self.api_base.trim_end_matches('/'), urlencoding::encode(query), per_page);
        self.get_json(&url)
    }

    pub fn assets_to_package_assets(&self, release: &GithubRelease) -> Vec<PackageAsset> {
        release.assets.iter().map(|a| PackageAsset::from_github_asset(a.name.clone(), a.browser_download_url.clone(), Some(a.size))).collect()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchResult {
    pub total_count: u64,
    pub incomplete_results: bool,
    pub items: Vec<GithubRepo>,
}

// Simple urlencoding
mod urlencoding {
    pub fn encode(input: &str) -> String {
        let mut out = String::new();
        for b in input.bytes() {
            match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
                b' ' => out.push_str("%20"),
                _ => out.push_str(&format!("%{:02X}", b)),
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse_repo_spec() {
        assert_eq!(GithubProvider::parse_repo_spec("user@repo"), Some(("user".to_string(), "repo".to_string())));
        assert_eq!(GithubProvider::parse_repo_spec("user/repo"), Some(("user".to_string(), "repo".to_string())));
        assert_eq!(GithubProvider::parse_repo_spec("https://github.com/user/repo"), Some(("user".to_string(), "repo".to_string())));
        assert_eq!(GithubProvider::parse_repo_spec("github.com/user/repo"), Some(("user".to_string(), "repo".to_string())));
        assert_eq!(GithubProvider::parse_repo_spec("not-a-repo"), None);
    }
}
