use crate::distro::Distribution;
use crate::config::Config;
use crate::cache::Cache;
use crate::github::GithubProvider;
use crate::flathub::FlathubProvider;
use crate::packages::{PackageAsset, ScoredAsset, score_asset};
use crate::github::GithubRelease;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct ResolveOptions {
    pub preferred_source: Option<String>, // "native", "flathub"
    pub github_override: Option<String>,  // user@repo
    pub allow_flatpak: bool,
    pub prefer_native: bool,
    pub yes: bool,
}

impl Default for ResolveOptions {
    fn default() -> Self {
        Self { preferred_source: None, github_override: None, allow_flatpak: true, prefer_native: true, yes: false }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResolvedSource {
    Github { owner: String, repo: String, tag: String, asset: PackageAsset, release_url: String },
    Flathub { app_id: String, name: String, summary: Option<String> },
    NativeRepo { package_name: String },
}

impl std::fmt::Display for ResolvedSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolvedSource::Github { owner, repo, tag, asset, .. } => write!(f, "GitHub {}/{} {} ({})", owner, repo, tag, asset.filename),
            ResolvedSource::Flathub { app_id, name, .. } => write!(f, "Flathub {} ({})", app_id, name),
            ResolvedSource::NativeRepo { package_name } => write!(f, "native repo {}", package_name),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResolutionResult {
    pub source: ResolvedSource,
    pub score: i32,
    pub distro: Distribution,
    pub reason: String,
}

pub struct Resolver {
    pub distro: Distribution,
    pub config: Config,
    pub cache: Cache,
    pub github: GithubProvider,
    pub flathub: FlathubProvider,
}

impl Resolver {
    pub fn new(distro: Distribution, config: Config) -> Self {
        let cache = Cache::from_config(&config);
        let github = GithubProvider::from_config(&config);
        let flathub = FlathubProvider::from_config(&config);
        Self { distro, config, cache, github, flathub }
    }

    /// Resolve a package spec string (either user@repo, user/repo, or generic name)
    pub fn resolve(&self, spec: &str, opts: &ResolveOptions) -> anyhow::Result<ResolutionResult> {
        // Check preferred source override
        if let Some(pref) = &opts.preferred_source {
            if pref.to_lowercase() == "flathub" {
                // Force flathub search
                return self.resolve_flathub(spec);
            } else if pref.to_lowercase() == "native" {
                // Force github native (if spec is repo) else search github then try native fallback?
                // We'll try github first and if fails error, don't fallback to flathub
                let gh = self.resolve_github(spec, opts)?;
                return Ok(gh);
            }
        }

        // If github_override provided, treat spec as package name but use override repo
        if let Some(override_repo) = &opts.github_override {
            if let Some((owner, repo)) = GithubProvider::parse_repo_spec(override_repo) {
                return self.resolve_github_repo(&owner, &repo, opts);
            } else {
                anyhow::bail!("Invalid --github value '{}', expected user@repo or user/repo", override_repo);
            }
        }

        // If spec looks like repo spec, resolve directly via github
        if let Some((owner, repo)) = GithubProvider::parse_repo_spec(spec) {
            // Try github first
            match self.resolve_github_repo(&owner, &repo, opts) {
                Ok(r) => return Ok(r),
                Err(e) => {
                    eprintln!("  GitHub resolution failed for {}/{}: {}", owner, repo, e);
                    // If allowed, try flathub fallback for the repo name part
                    if opts.allow_flatpak && self.config.flathub.enabled {
                        println!("  Falling back to Flathub search for '{}'...", repo);
                        if let Ok(fl) = self.resolve_flathub(&repo) {
                            return Ok(fl);
                        }
                    }
                    return Err(e);
                }
            }
        }

        // Generic name: search GitHub then Flathub
        // Try GitHub search
        let mut best_github: Option<ResolutionResult> = None;
        if opts.prefer_native {
            match self.search_and_resolve_github(spec, opts) {
                Ok(r) => best_github = Some(r),
                Err(e) => eprintln!("  GitHub search found no compatible native package for '{}': {}", spec, e),
            }
        }

        if let Some(gh) = best_github {
            // If we have a good github result with high score (>50), prefer it
            if gh.score > 50 || !opts.allow_flatpak {
                return Ok(gh);
            }
            // Otherwise check flathub as fallback maybe better
            if opts.allow_flatpak && self.config.flathub.enabled {
                match self.resolve_flathub(spec) {
                    Ok(fl) => {
                        // Decide: if github score is native + arch correct, keep github, else flathub
                        // Already we know github score <=50 means not great
                        println!("  GitHub result score {} not ideal, considering Flathub fallback", gh.score);
                        return Ok(fl);
                    },
                    Err(_) => return Ok(gh), // no flathub, keep github
                }
            } else {
                return Ok(gh);
            }
        }

        // No github result, try flathub
        if opts.allow_flatpak && self.config.flathub.enabled {
            match self.resolve_flathub(spec) {
                Ok(fl) => return Ok(fl),
                Err(e) => eprintln!("  Flathub search also failed: {}", e),
            }
        }

        anyhow::bail!("No compatible package found for '{}' on {} {}. Searched GitHub{} and Flathub. Try specifying a GitHub repo via user@repo.", spec, self.distro.id, self.distro.architecture, if opts.allow_flatpak { "" } else { " (flatpak disabled)" })
    }

    fn resolve_github(&self, spec: &str, opts: &ResolveOptions) -> anyhow::Result<ResolutionResult> {
        if let Some((owner, repo)) = GithubProvider::parse_repo_spec(spec) {
            self.resolve_github_repo(&owner, &repo, opts)
        } else {
            self.search_and_resolve_github(spec, opts)
        }
    }

    pub fn resolve_github_repo(&self, owner: &str, repo: &str, _opts: &ResolveOptions) -> anyhow::Result<ResolutionResult> {
        // Check cache first
        let cache_key = format!("github_release_{}_{}", owner, repo);
        let cached: Option<CachedRelease> = self.cache.get_metadata(&cache_key);
        let release: GithubRelease = if let Some(cached) = cached {
            // Use cached if available, but we still want to verify it's not stale?
            // For now use cached release directly
            // Convert cached to GithubRelease? We store whole release
            cached.release
        } else {
            let rel = self.github.get_latest_stable_release(owner, repo)?;
            // Cache it
            let cached = CachedRelease { release: rel.clone(), fetched_at: chrono::Utc::now().to_rfc3339() };
            let _ = self.cache.set_metadata(&cache_key, &cached);
            rel
        };

        let assets = self.github.assets_to_package_assets(&release);
        if assets.is_empty() {
            anyhow::bail!("Release {} has no assets", release.tag_name);
        }

        let scored = self.score_assets(assets, release.prerelease);
        // Find best compatible
        let best = scored.iter().filter(|s| s.is_compatible && s.score > 0).max_by_key(|s| s.score);
        if let Some(best) = best {
            // Also check checksum asset
            let checksum_url = release.assets.iter().find(|a| {
                let lower = a.name.to_lowercase();
                lower.contains("checksum") || lower.ends_with(".sha256") || lower == "checksums.txt"
            }).map(|a| a.browser_download_url.clone());

            // For now ignore checksum_url but could pass along
            let _ = checksum_url;

            Ok(ResolutionResult {
                source: ResolvedSource::Github {
                    owner: owner.to_string(),
                    repo: repo.to_string(),
                    tag: release.tag_name.clone(),
                    asset: best.asset.clone(),
                    release_url: release.html_url.clone(),
                },
                score: best.score,
                distro: self.distro.clone(),
                reason: best.reasons.join(", "),
            })
        } else {
            // No compatible, show diagnostics
            let diagnostics: Vec<String> = scored.iter().map(|s| format!("{}: score {} ({}) compat={} - {}", s.asset.filename, s.score, s.asset.format, s.is_compatible, s.reasons.join("; "))).collect();
            eprintln!("  Analyzed {} assets:", scored.len());
            for d in &diagnostics { eprintln!("    - {}", d); }
            anyhow::bail!("No compatible native package found for {}/{} on {} ({}). Available assets do not match system arch/format. Use --default flathub to force Flatpak.", owner, repo, self.distro.id, self.distro.architecture)
        }
    }

    fn search_and_resolve_github(&self, query: &str, opts: &ResolveOptions) -> anyhow::Result<ResolutionResult> {
        // Search GitHub repos
        println!("  Searching GitHub for '{}'...", query);
        let search = self.github.search_repos(query, 5)?;
        if search.items.is_empty() {
            anyhow::bail!("GitHub search returned no results for '{}'", query);
        }
        println!("  Found {} repositories, checking releases...", search.items.len());
        let mut best_global: Option<ResolutionResult> = None;
        let mut best_score = i32::MIN;
        let mut attempts = 0;
        for repo in search.items.iter().take(3) {
            let owner = repo.owner.login.clone();
            let name = repo.name.clone();
            print!("  Checking {}/{} ... ", owner, name);
            match self.resolve_github_repo(&owner, &name, opts) {
                Ok(res) => {
                    println!("score {}", res.score);
                    if res.score > best_score {
                        best_score = res.score;
                        best_global = Some(res);
                    }
                    attempts += 1;
                    // If we found a high score native package, break early
                    if best_score >= 150 { break; }
                },
                Err(e) => {
                    println!("no compatible ({})", e);
                }
            }
        }
        if let Some(best) = best_global {
            Ok(best)
        } else {
            if attempts == 0 {
                anyhow::bail!("No compatible packages among top GitHub results for '{}'", query);
            } else {
                anyhow::bail!("Checked {} repos but none had compatible assets for your distro", attempts);
            }
        }
    }

    pub fn resolve_flathub(&self, query: &str) -> anyhow::Result<ResolutionResult> {
        println!("  Searching Flathub for '{}'...", query);
        // First try API
        match self.flathub.search(query) {
            Ok(hits) => {
                if hits.is_empty() {
                    // Try local flatpak search as fallback
                    if let Ok(local) = FlathubProvider::search_local_flatpak(query) {
                        if !local.is_empty() {
                            let app_id = local[0].clone();
                            return Ok(ResolutionResult {
                                source: ResolvedSource::Flathub { app_id: app_id.clone(), name: app_id.clone(), summary: None },
                                score: 5,
                                distro: self.distro.clone(),
                                reason: "Flathub local search".to_string(),
                            });
                        }
                    }
                    anyhow::bail!("No Flathub results for '{}'", query);
                }
                // If multiple, pick best? For now take first hit, but indicate choices
                if hits.len() > 1 {
                    println!("  Found {} Flathub packages:", hits.len());
                    for (i, hit) in hits.iter().enumerate().take(5) {
                        println!("    {}. {} ({}) - {}", i+1, hit.name, hit.app_id, hit.summary.as_deref().unwrap_or(""));
                    }
                    // Choose first for now; in interactive mode could prompt
                    println!("  Selecting first result: {}", hits[0].app_id);
                } else {
                    println!("  Found Flathub: {} ({})", hits[0].name, hits[0].app_id);
                }
                let chosen = &hits[0];
                Ok(ResolutionResult {
                    source: ResolvedSource::Flathub { app_id: chosen.app_id.clone(), name: chosen.name.clone(), summary: chosen.summary.clone() },
                    score: 5,
                    distro: self.distro.clone(),
                    reason: "Flathub fallback".to_string(),
                })
            },
            Err(e) => {
                // Try local flatpak search fallback
                if let Ok(local) = FlathubProvider::search_local_flatpak(query) {
                    if !local.is_empty() {
                        let app_id = local[0].clone();
                        return Ok(ResolutionResult {
                            source: ResolvedSource::Flathub { app_id: app_id.clone(), name: app_id.clone(), summary: None },
                            score: 5,
                            distro: self.distro.clone(),
                            reason: "Flathub local fallback".to_string(),
                        });
                    }
                }
                Err(anyhow::anyhow!("Flathub search failed: {}", e))
            }
        }
    }

    fn score_assets(&self, assets: Vec<PackageAsset>, is_prerelease: bool) -> Vec<ScoredAsset> {
        let is_stable = !is_prerelease;
        assets.iter().map(|a| score_asset(a, &self.distro, is_stable)).collect()
    }

    pub fn list_scored_for_repo(&self, owner: &str, repo: &str) -> anyhow::Result<Vec<ScoredAsset>> {
        let release = self.github.get_latest_stable_release(owner, repo)?;
        let assets = self.github.assets_to_package_assets(&release);
        Ok(self.score_assets(assets, !release.prerelease))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedRelease {
    release: GithubRelease,
    fetched_at: String,
}
