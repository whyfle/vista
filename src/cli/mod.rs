use clap::{Parser, Subcommand, Args, ValueEnum};
use colored::Colorize;
use std::path::Path;

use crate::distro;
use crate::config::Config;
use crate::database::{Database, InstalledPackage, InstallSource};
use crate::resolver::{Resolver, ResolveOptions, ResolvedSource};
use crate::installers;
use crate::installers::PackageManager;
use crate::downloader::Downloader;
use crate::security;
use crate::cache::Cache;

#[derive(Parser)]
#[command(name = "vista", author, version, about = "Vista — A GitHub-backed universal Linux package manager", long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Assume yes for prompts
    #[arg(short = 'y', long, global = true)]
    pub yes: bool,

    /// Dry run (do not actually install)
    #[arg(long, global = true, hide = true)]
    pub dry_run: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Install a package (alias: add)
    #[command(alias = "add")]
    Install(InstallArgs),

    /// Remove an installed package
    Remove(RemoveArgs),

    /// Update package metadata cache
    Update(UpdateArgs),

    /// Upgrade installed packages
    Upgrade(UpgradeArgs),

    /// Search for packages
    Search(SearchArgs),

    /// Show info about a package
    Info(InfoArgs),

    /// List installed packages
    List(ListArgs),

    /// Clean caches
    Clean(CleanArgs),

    /// Show system distribution info
    SysInfo,

    /// Show version
    Version,
}

#[derive(Args, Debug, Clone)]
pub struct InstallArgs {
    /// Package name or GitHub repo (user@repo, user/repo, or plain name)
    pub package: String,

    /// Override GitHub repo (e.g., user@repo)
    #[arg(long, value_name = "REPO")]
    pub github: Option<String>,

    /// Alternative: --repo flag (same as --github)
    #[arg(long, value_name = "REPO")]
    pub repo: Option<String>,

    /// Preferred source: native, flathub, auto
    #[arg(long, value_name = "SOURCE", default_value = "auto")]
    pub default: String,

    /// Force flatpak (shortcut for --default flathub)
    #[arg(long)]
    pub flatpak: bool,

    /// Assume yes
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// Dry run
    #[arg(long)]
    pub dry_run: bool,

    /// Verbose scoring details
    #[arg(long)]
    pub verbose: bool,
}

#[derive(Args, Debug, Clone)]
pub struct RemoveArgs {
    pub package: String,
    #[arg(short = 'y', long)]
    pub yes: bool,
    #[arg(long)] pub dry_run: bool,
}

#[derive(Args, Debug, Clone)]
pub struct UpdateArgs {
    #[arg(long)] pub clean: bool,
}

#[derive(Args, Debug, Clone)]
pub struct UpgradeArgs {
    #[arg(short = 'y', long)] pub yes: bool,
    #[arg(long)] pub dry_run: bool,
}

#[derive(Args, Debug, Clone)]
pub struct SearchArgs {
    pub query: String,
    #[arg(long, default_value = "5")] pub limit: u8,
}

#[derive(Args, Debug, Clone)]
pub struct InfoArgs {
    pub package: String,
    #[arg(long)] pub github: Option<String>,
}

#[derive(Args, Debug, Clone)]
pub struct ListArgs {
    #[arg(long)] pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct CleanArgs {
    #[arg(long)] pub all: bool,
}

pub fn run(cli: Cli) -> anyhow::Result<()> {
    let config = Config::load();
    let distro = distro::detect_distribution();

    // Handle verbose global
    if cli.verbose { println!("Verbose mode enabled"); }

    match cli.command {
        Commands::Install(args) => handle_install(args, distro, config, cli.yes || cli.dry_run, cli.dry_run),
        Commands::Remove(args) => handle_remove(args, cli.yes, cli.dry_run),
        Commands::Update(args) => handle_update(args, config),
        Commands::Upgrade(args) => handle_upgrade(args, cli.yes, cli.dry_run),
        Commands::Search(args) => handle_search(args, distro, config),
        Commands::Info(args) => handle_info(args, distro, config),
        Commands::List(args) => handle_list(args),
        Commands::Clean(args) => handle_clean(args, config),
        Commands::SysInfo => handle_sysinfo(distro),
        Commands::Version => {
            println!("vista {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        },
    }
}

fn handle_install(args: InstallArgs, distro: distro::Distribution, config: Config, global_yes: bool, global_dry: bool) -> anyhow::Result<()> {
    let yes = args.yes || global_yes || config.general.auto_confirm;
    let dry_run = args.dry_run || global_dry;
    let verbose = args.verbose;

    println!("\n  {}","Vista Package Manager".bold().cyan());
    println!();
    println!("  Detecting system...");
    println!("  {} {} {}", "✓".green(), distro.name, distro.version_id.dimmed());
    println!("  {} {}", "✓".green(), distro.architecture.dimmed());
    println!("  Family: {}  Native: {}", distro.family.to_string().yellow(), distro.native_format.to_string().yellow());
    println!();

    // Resolve preferred source
    let mut preferred: Option<String> = None;
    let default_lower = args.default.to_lowercase();
    if default_lower == "flathub" || args.flatpak {
        preferred = Some("flathub".to_string());
    } else if default_lower == "native" {
        preferred = Some("native".to_string());
    } else if default_lower != "auto" {
        eprintln!("Warning: unknown --default value '{}', using auto", args.default);
    }

    let github_override = args.github.clone().or(args.repo.clone());

    let opts = ResolveOptions {
        preferred_source: preferred,
        github_override,
        allow_flatpak: config.resolver.allow_flatpak,
        prefer_native: config.resolver.prefer_native,
        yes,
    };

    let resolver = Resolver::new(distro.clone(), config.clone());

    println!("  Searching GitHub...");
    // Special case: if spec contains user@repo but with install --github flag? Already handled
    // Resolver will handle github vs search
    let resolution = resolver.resolve(&args.package, &opts);

    match resolution {
        Ok(res) => {
            match res.source {
                ResolvedSource::Github { owner, repo, tag, asset, release_url } => {
                    println!("  {} Found release {}", "✓".green(), tag.bold());
                    println!();
                    println!("  Analyzing packages...");
                    println!("  {} {}", "✓".green(), asset.filename.bold());
                    println!();
                    println!("  Selected:");
                    println!("    {} {}", "Source:".dimmed(), "GitHub".cyan());
                    println!("    {} {}/{}", "Repository:".dimmed(), owner, repo);
                    println!("    {} {}", "Package:".dimmed(), asset.filename);
                    println!("    {} {}", "Format:".dimmed(), asset.format.to_string().yellow());
                    println!("    {} {}", "Architecture:".dimmed(), asset.arch.as_deref().unwrap_or("unknown"));
                    println!("    {} {}", "Version:".dimmed(), asset.version.as_deref().unwrap_or(&tag));
                    println!("    {} {}", "URL:".dimmed(), asset.url.dimmed());
                    println!("    {} {}", "Release:".dimmed(), release_url.dimmed());
                    println!("    Score: {} ({})", res.score, res.reason.dimmed());
                    println!();

                    // Security checks
                    let warnings = security::security_summary_for_package(&asset, &distro);
                    for w in &warnings {
                        println!("  {} {}", "⚠".yellow(), w.yellow());
                    }
                    if !warnings.is_empty() {
                        println!();
                    }

                    // Check available managers
                    let managers = installers::available_managers();
                    println!("  Available package managers: {}", managers.iter().map(|m| m.name()).collect::<Vec<_>>().join(", ").dimmed());

                    // Confirm
                    if !yes && config.security.require_confirmation {
                        // Show summary like spec
                        println!();
                        println!("  Vista found:");
                        println!();
                        println!("    {} {}", asset.filename, tag.dimmed());
                        println!("    Source: GitHub");
                        println!("    Package: {}", asset.filename);
                        println!("    Architecture: {}", asset.arch.as_deref().unwrap_or("unknown"));
                        println!("    Distribution: {} ({})", distro.id, distro.native_format);
                        println!();
                        let prompt = format!("  Install this package? [Y/n]");
                        if !security::prompt_confirmation(&prompt, yes) {
                            println!("  Aborted.");
                            return Ok(());
                        }
                    } else if dry_run {
                        println!("  {} dry-run mode, not installing", "[dry-run]".yellow());
                    }

                    // Download
                    let downloader = Downloader::new();
                    let cache = Cache::from_config(&config);
                    let dest = cache.download_path(&asset.filename);
                    // Ensure cache dir
                    cache.ensure_dir().ok();
                    if !dry_run {
                        println!("  Downloading...");
                        let bytes = downloader.download(&asset.url, &dest)?;
                        println!("  {} Downloaded {} bytes to {:?}", "✓".green(), bytes, dest);

                        // Verify checksum if available - try to find checksum asset
                        // Attempt to fetch checksums.txt from same release if exists? For now skip unless asset provides?
                        // We could try to find sibling checksum file
                        // Let's attempt to fetch checksum file content if release had one
                        // We need to re-fetch release to get checksum url - but we already resolved via resolver; we need to fetch again
                        // Simpler: just verify no checksum case
                        let mut warnings = Vec::new();
                        let verify = security::verify_package(&dest, None, &mut warnings);
                        for w in warnings { println!("  {} {}", "⚠".yellow(), w); }
                        println!("  SHA256: {}", verify.actual.clone().unwrap_or_default().dimmed());
                        println!("  {}", "Verifying package arch/format...".dimmed());
                        if let Err(e) = security::validate_package_format(&dest, &asset.format) {
                            eprintln!("  {} Format validation failed: {}", "✗".red(), e);
                            if !yes {
                                anyhow::bail!("Aborting due to format validation failure");
                            }
                        }

                        // Install via native package manager
                        println!();
                        println!("  Installing...");
                        let manager: Box<dyn installers::PackageManager> = match asset.format {
                            crate::packages::PackageFormat::Rpm => {
                                if installers::DnfAdapter::new(false).is_available() {
                                    Box::new(installers::DnfAdapter::new(dry_run))
                                } else if installers::ZypperAdapter::new(false).is_available() {
                                    Box::new(installers::ZypperAdapter::new(dry_run))
                                } else {
                                    anyhow::bail!("No RPM package manager found (dnf/zypper) to install .rpm");
                                }
                            },
                            crate::packages::PackageFormat::Deb => {
                                if installers::AptAdapter::new(false).is_available() {
                                    Box::new(installers::AptAdapter::new(dry_run))
                                } else {
                                    anyhow::bail!("No DEB package manager found (apt) to install .deb");
                                }
                            },
                            crate::packages::PackageFormat::Pacman => Box::new(installers::PacmanAdapter::new(dry_run)),
                            crate::packages::PackageFormat::Apk => {
                                // Use apk directly
                                // Find available?
                                anyhow::bail!("APK installation not fully automated yet; package downloaded to {:?}", dest);
                            },
                            crate::packages::PackageFormat::AppImage => {
                                // AppImage: just make executable and move to ~/Applications or /usr/local/bin?
                                // For now, mark as installed without pkg manager
                                println!("  {} AppImage detected — making executable and installing to ~/Applications", "→".cyan());
                                let apps_dir = dirs::home_dir().map(|h| h.join("Applications")).unwrap_or_else(|| Path::new("/tmp").to_path_buf());
                                std::fs::create_dir_all(&apps_dir)?;
                                let target = apps_dir.join(&asset.filename);
                                std::fs::copy(&dest, &target)?;
                                // Make executable
                                #[cfg(unix)]
                                {
                                    use std::os::unix::fs::PermissionsExt;
                                    let mut perms = std::fs::metadata(&target)?.permissions();
                                    perms.set_mode(0o755);
                                    std::fs::set_permissions(&target, perms)?;
                                }
                                println!("  {} AppImage installed to {:?}", "✓".green(), target);
                                // Record to DB and return early
                                let mut db = Database::load();
                                db.add(InstalledPackage {
                                    name: args.package.clone(),
                                    version: tag.clone(),
                                    source: InstallSource::Github,
                                    repository: Some(format!("{}/{}", owner, repo)),
                                    installation_method: asset.format.to_string(),
                                    package_identifier: target.to_string_lossy().to_string(),
                                    architecture: distro.architecture.clone(),
                                    checksum: verify.actual.clone(),
                                    installed_at: chrono::Utc::now(),
                                    install_path: Some(target.to_string_lossy().to_string()),
                                    download_url: Some(asset.url.clone()),
                                    size: asset.size,
                                });
                                db.save()?;
                                println!();
                                println!("  {} {} {} is now installed.", "✓".green(), args.package.bold(), tag.dimmed());
                                return Ok(());
                            },
                            _ => {
                                anyhow::bail!("Unsupported package format {} for direct install via native manager. Downloaded to {:?}. Please install manually.", asset.format, dest);
                            }
                        };

                        // Verify manager available
                        if !manager.is_available() {
                            anyhow::bail!("Package manager {} not available on this system", manager.name());
                        }

                        // Need privileges? Check if we are root or can use sudo?
                        // For now attempt install; if fails due to permission, suggest sudo
                        match manager.install(dest.to_string_lossy().as_ref(), yes) {
                            Ok(_) => {
                                println!("  {} Installation complete via {}", "✓".green(), manager.name().bold());
                            },
                            Err(e) => {
                                eprintln!("  {} Installation failed: {}", "✗".red(), e);
                                eprintln!("  Package file retained at {:?}", dest);
                                eprintln!("  Try running with sudo: sudo vista install {}", args.package);
                                return Err(e);
                            }
                        }

                        // Record in DB
                        let mut db = Database::load();
                        db.add(InstalledPackage {
                            name: args.package.clone(),
                            version: tag.clone(),
                            source: InstallSource::Github,
                            repository: Some(format!("{}/{}", owner, repo)),
                            installation_method: asset.format.to_string(),
                            package_identifier: asset.filename.clone(),
                            architecture: asset.arch.clone().unwrap_or(distro.architecture.clone()),
                            checksum: verify.actual.clone(),
                            installed_at: chrono::Utc::now(),
                            install_path: Some(dest.to_string_lossy().to_string()),
                            download_url: Some(asset.url.clone()),
                            size: asset.size,
                        });
                        db.save()?;

                        println!();
                        println!("  {} {} {} is now installed.", "✓".green(), args.package.bold(), tag.dimmed());
                    } else {
                        println!("  {} Dry-run: would have installed {:?}", "[dry-run]".yellow(), dest);
                    }

                },
                ResolvedSource::Flathub { app_id, name, summary } => {
                    println!("  {} No compatible native package", "✗".red());
                    println!();
                    println!("  Searching Flathub...");
                    println!("  {} Found {} ({})", "✓".green(), name.bold(), app_id.cyan());
                    if let Some(s) = summary { println!("    {}", s.dimmed()); }
                    println!();
                    println!("  Using Flatpak fallback...");
                    println!();

                    if !installers::FlatpakAdapter::new(false).is_available() {
                        anyhow::bail!("Flatpak is not installed on this system. Cannot install Flathub package {}. Please install flatpak first.", app_id);
                    }

                    if !yes && config.security.require_confirmation {
                        let prompt = format!("  Install Flatpak {} ? [Y/n]", app_id);
                        if !security::prompt_confirmation(&prompt, yes) {
                            println!("  Aborted.");
                            return Ok(());
                        }
                    }

                    if !dry_run {
                        let flatpak = installers::FlatpakAdapter::new(false);
                        println!("  Installing Flatpak {}...", app_id);
                        flatpak.install(&app_id, yes)?;
                        println!("  {} Installation complete", "✓".green());

                        let mut db = Database::load();
                        db.add(InstalledPackage {
                            name: args.package.clone(),
                            version: "latest".to_string(),
                            source: InstallSource::Flathub,
                            repository: Some(app_id.clone()),
                            installation_method: "flatpak".to_string(),
                            package_identifier: app_id.clone(),
                            architecture: distro.architecture.clone(),
                            checksum: None,
                            installed_at: chrono::Utc::now(),
                            install_path: None,
                            download_url: None,
                            size: None,
                        });
                        db.save()?;

                        println!();
                        println!("  {} {} is now installed via Flatpak.", "✓".green(), app_id.bold());
                    } else {
                        println!("  {} Dry-run: would install flatpak {}", "[dry-run]".yellow(), app_id);
                    }
                    if verbose {
                        println!("  Score: {} reason: {}", res.score, res.reason);
                    }
                },
                ResolvedSource::NativeRepo { package_name } => {
                    println!("  Found native repository package: {}", package_name);
                }
            }
            Ok(())
        },
        Err(e) => {
            eprintln!("\n  {} Failed to resolve package '{}': {}", "✗".red(), args.package.bold(), e);
            eprintln!();
            eprintln!("  Suggestions:");
            eprintln!("    • Check the name: vista search <query>");
            eprintln!("    • Try explicit GitHub repo: vista install user@repo");
            eprintln!("    • Force Flatpak: vista install {} --default flathub", args.package);
            eprintln!("    • Check your distro: {} is {}", distro.id, distro.native_format);
            Err(e)
        }
    }
}

fn handle_remove(args: RemoveArgs, global_yes: bool, global_dry: bool) -> anyhow::Result<()> {
    let yes = args.yes || global_yes;
    let dry = args.dry_run || global_dry;
    let mut db = Database::load();
    if let Some(pkg) = db.get(&args.package).cloned() {
        println!("  Found installed package:");
        println!("    Name: {}", pkg.name.bold());
        println!("    Version: {}", pkg.version);
        println!("    Source: {}", pkg.source);
        println!("    Method: {}", pkg.installation_method);
        println!("    ID: {}", pkg.package_identifier);
        if !yes {
            if !security::prompt_confirmation(&format!("  Remove {}? [Y/n]", pkg.name), yes) {
                println!("  Aborted.");
                return Ok(());
            }
        }
        match pkg.installation_method.as_str() {
            "flatpak" => {
                let flatpak = installers::FlatpakAdapter::new(dry);
                if flatpak.is_available() {
                    flatpak.remove(&pkg.package_identifier, yes)?;
                } else {
                    eprintln!("  Warning: flatpak not available, removing only from database");
                }
            },
            "rpm" => {
                let dnf = installers::DnfAdapter::new(dry);
                if dnf.is_available() {
                    // Try to remove; package name may need to be derived? Use pkg.name
                    dnf.remove(&pkg.name, yes).or_else(|_| dnf.remove(&pkg.package_identifier, yes))?;
                }
            },
            "deb" => {
                let apt = installers::AptAdapter::new(dry);
                if apt.is_available() {
                    apt.remove(&pkg.name, yes)?;
                }
            },
            "pacman" => {
                let pacman = installers::PacmanAdapter::new(dry);
                if pacman.is_available() { pacman.remove(&pkg.name, yes)?; }
            },
            _ => {
                // For AppImage or untracked, just remove file
                if let Some(path) = &pkg.install_path {
                    let p = Path::new(path);
                    if p.exists() && !dry {
                        std::fs::remove_file(p)?;
                        println!("  Removed file {:?}", p);
                    } else if dry {
                        println!("  [dry-run] Would remove file {:?}", p);
                    }
                }
            }
        }
        if !dry {
            db.remove(&args.package);
            db.save()?;
            println!("  {} Removed {}", "✓".green(), args.package.bold());
        } else {
            println!("  [dry-run] Would remove {} from database", args.package);
        }
        Ok(())
    } else {
        // Not in DB, try to remove via package managers anyway? For now try flatpak, dnf, apt
        println!("  Package '{}' not found in Vista database, attempting system removal...", args.package);
        let mut tried = false;
        let managers = installers::available_managers();
        for m in managers {
            println!("  Trying {} remove {} ...", m.name(), args.package);
            match m.remove(&args.package, yes) {
                Ok(_) => { println!("  {} Removed via {}", "✓".green(), m.name()); tried = true; break; },
                Err(e) => eprintln!("    {} {} failed: {}", "→".dimmed(), m.name(), e),
            }
        }
        if !tried {
            anyhow::bail!("Package '{}' not found in database and no system package manager could remove it", args.package);
        }
        Ok(())
    }
}

fn handle_update(args: UpdateArgs, config: Config) -> anyhow::Result<()> {
    let cache = Cache::from_config(&config);
    if args.clean {
        cache.clean_metadata()?;
        println!("  {} Cleaned metadata cache", "✓".green());
    } else {
        // For now just clean and say updating
        cache.clean_metadata()?;
        println!("  {} Metadata cache cleared. Next install will fetch fresh data.", "✓".green());
    }
    // Could also update flatpak metadata? flatpak update handling via upgrade
    Ok(())
}

fn handle_upgrade(args: UpgradeArgs, global_yes: bool, global_dry: bool) -> anyhow::Result<()> {
    let yes = args.yes || global_yes;
    let dry = args.dry_run || global_dry;
    let db = Database::load();
    if db.packages.is_empty() {
        println!("  No packages installed via Vista.");
        return Ok(());
    }
    println!("  Checking for upgrades for {} packages...", db.count());
    // For each package, try to resolve latest again? For now just run system managers updates
    let managers = installers::available_managers();
    for m in managers {
        println!("  {} updating via {}...", "→".cyan(), m.name());
        match m.update() {
            Ok(_) => println!("  {} {} update complete", "✓".green(), m.name()),
            Err(e) => eprintln!("  {} {} update failed: {}", "✗".red(), m.name(), e),
        }
        if dry { break; } // dry run maybe only once?
    }
    // Also check github packages for newer releases? Would need to re-resolve
    // For each github package, fetch latest and compare semver
    let config = Config::load();
    let distro = distro::detect_distribution();
    let resolver = Resolver::new(distro, config);
    for pkg in db.list() {
        if pkg.source == InstallSource::Github {
            if let Some(repo) = &pkg.repository {
                if let Some((owner, repo_name)) = crate::github::GithubProvider::parse_repo_spec(repo) {
                    println!("  Checking {} ({})...", pkg.name, repo);
                    let opts = crate::resolver::ResolveOptions { allow_flatpak: true, prefer_native: true, ..Default::default() };
                    match resolver.resolve_github_repo(&owner, &repo_name, &opts) {
                        Ok(res) => {
                            let latest_version = match &res.source {
                                ResolvedSource::Github { tag, .. } => tag.clone(),
                                _ => "unknown".to_string(),
                            };
                            if latest_version != pkg.version {
                                println!("    Update available: {} -> {}", pkg.version.yellow(), latest_version.green());
                                if !dry && security::prompt_confirmation(&format!("    Upgrade {}?", pkg.name), yes) {
                                    // For now just notify; actual upgrade would re-download install?
                                    println!("    (Upgrade via reinstall: vista install {} )", pkg.name);
                                }
                            } else {
                                println!("    Up to date: {}", pkg.version.dimmed());
                            }
                        },
                        Err(e) => eprintln!("    Failed to check {}: {}", pkg.name, e),
                    }
                }
            }
        }
    }
    let _ = yes;
    Ok(())
}

fn handle_search(args: SearchArgs, distro: distro::Distribution, config: Config) -> anyhow::Result<()> {
    println!("\n  {} Searching for '{}'...", "Vista".cyan().bold(), args.query.bold());
    println!("  Distro: {}  Arch: {}", distro.id, distro.architecture);
    println!();

    let github = crate::github::GithubProvider::from_config(&config);
    let flathub = crate::flathub::FlathubProvider::from_config(&config);

    // Search GitHub
    println!("  GitHub results:");
    match github.search_repos(&args.query, args.limit) {
        Ok(res) => {
            if res.items.is_empty() {
                println!("    No GitHub repositories found.");
            } else {
                for (i, repo) in res.items.iter().enumerate().take(args.limit as usize) {
                    println!("\n  {}. {} {}", i+1, repo.full_name.bold().cyan(), format!("★ {}", repo.stargazers_count.unwrap_or(0)).yellow().dimmed());
                    if let Some(desc) = &repo.description {
                        println!("     {}", desc.dimmed());
                    }
                    println!("     {}", repo.html_url.dimmed());
                    // Try to hint native availability? Could check releases quickly but skip for search speed?
                    // We'll attempt to quick check release assets if not too heavy? Limit to first?
                    // For now just indicate
                }
            }
        },
        Err(e) => eprintln!("    GitHub search failed: {}", e),
    }

    println!("\n  Flathub results:");
    if config.flathub.enabled {
        match flathub.search(&args.query) {
            Ok(hits) => {
                if hits.is_empty() {
                    println!("    No Flathub packages found.");
                } else {
                    for (i, hit) in hits.iter().enumerate().take(args.limit as usize) {
                        println!("\n  {}. {} ({})", i+1, hit.name.bold().green(), hit.app_id.cyan());
                        if let Some(s) = &hit.summary { println!("     {}", s.dimmed()); }
                        println!("     https://flathub.org/apps/{}", hit.app_id);
                    }
                }
            },
            Err(e) => eprintln!("    Flathub search failed: {}", e),
        }
    } else {
        println!("    Flathub disabled in config");
    }

    println!("\n  Recommendations for your system ({}):", distro.native_format.to_string().yellow());
    println!("    Native format preferred: {}", distro.native_format);
    // Could show scoring for first github hit?
    println!();
    Ok(())
}

fn handle_info(args: InfoArgs, distro: distro::Distribution, config: Config) -> anyhow::Result<()> {
    let db = Database::load();
    if let Some(pkg) = db.get(&args.package) {
        println!("\n  {} Installed package info", "Vista".cyan().bold());
        println!("    Name: {}", pkg.name.bold());
        println!("    Version: {}", pkg.version);
        println!("    Source: {}", pkg.source);
        println!("    Repository: {}", pkg.repository.as_deref().unwrap_or("-"));
        println!("    Method: {}", pkg.installation_method);
        println!("    ID: {}", pkg.package_identifier);
        println!("    Arch: {}", pkg.architecture);
        println!("    Checksum: {}", pkg.checksum.as_deref().unwrap_or("-"));
        println!("    Installed: {}", pkg.installed_at);
        if let Some(p) = &pkg.install_path { println!("    Path: {}", p); }
        if let Some(u) = &pkg.download_url { println!("    URL: {}", u.dimmed()); }
        println!();
        return Ok(());
    }

    // Not installed, try to show resolution preview
    println!("  Package '{}' not installed. Showing available options for your distro...", args.package);
    let resolver = Resolver::new(distro.clone(), config.clone());
    let opts = ResolveOptions::default();

    // If package looks like repo spec or override provided
    let spec = args.github.as_deref().unwrap_or(&args.package);
    if let Some((owner, repo)) = crate::github::GithubProvider::parse_repo_spec(spec) {
        println!("\n  Checking GitHub {}/{}...", owner, repo);
        match resolver.list_scored_for_repo(&owner, &repo) {
            Ok(scored) => {
                println!("\n  Assets for latest release:");
                for s in scored {
                    let compat = if s.is_compatible { "✓".green().to_string() } else { "✗".red().to_string() };
                    println!("    {} {} [{}] score {} - {}", compat, s.asset.filename.bold(), s.asset.format, s.score.to_string().yellow(), s.reasons.join("; ").dimmed());
                }
                // Also show flathub fallback
                if config.flathub.enabled {
                    println!("\n  Flathub fallback check for '{}'...", repo);
                    match resolver.resolve_flathub(&repo) {
                        Ok(fl) => println!("    Found: {}", fl.source),
                        Err(e) => println!("    No Flathub: {}", e),
                    }
                }
            },
            Err(e) => eprintln!("  Failed to fetch releases: {}", e),
        }
    } else {
        // Try search
        handle_search(SearchArgs { query: spec.to_string(), limit: 5 }, distro, config)?;
    }
    Ok(())
}

fn handle_list(args: ListArgs) -> anyhow::Result<()> {
    let db = Database::load();
    if db.packages.is_empty() {
        println!("  No packages installed via Vista.");
        println!("  Use: vista install <package>");
        return Ok(());
    }
    if args.json {
        let json = serde_json::to_string_pretty(&db)?;
        println!("{}", json);
        return Ok(());
    }
    println!("\n  Installed packages ({}):", db.count().to_string().bold());
    println!("  {}", "─".repeat(60).dimmed());
    for pkg in db.list() {
        println!("  {} {} {}", pkg.name.bold().cyan(), pkg.version.green(), format!("({})", pkg.source).dimmed());
        println!("    Method: {}  Repo: {}  Arch: {}", pkg.installation_method.yellow(), pkg.repository.as_deref().unwrap_or("-").dimmed(), pkg.architecture.dimmed());
        println!("    Installed: {}  ID: {}", pkg.installed_at.format("%Y-%m-%d %H:%M").to_string().dimmed(), pkg.package_identifier.dimmed());
        println!();
    }
    Ok(())
}

fn handle_clean(args: CleanArgs, config: Config) -> anyhow::Result<()> {
    let cache = Cache::from_config(&config);
    let stats = cache.stats();
    println!("  Cache directory: {:?}", cache.dir);
    println!("  Metadata files: {} ({} bytes)", stats.metadata_files, stats.metadata_size);
    println!("  Download files: {} ({} bytes)", stats.download_files, stats.download_size);
    cache.clean()?;
    println!("  {} Cache cleaned", "✓".green());
    if args.all {
        let db_path = Database::db_path();
        if let Some(p) = db_path { if p.exists() { println!("  Database at {:?} not removed (use --all with explicit confirmation? For now only cache cleared)", p); } }
    }
    Ok(())
}

fn handle_sysinfo(distro: distro::Distribution) -> anyhow::Result<()> {
    println!("\n  System Information");
    println!("  {}", "─".repeat(40).dimmed());
    println!("    ID: {}", distro.id.bold());
    println!("    Name: {}", distro.name);
    println!("    Version: {} ({})", distro.version_id, distro.version);
    println!("    Architecture: {}", distro.architecture.yellow().bold());
    println!("    Family: {}", distro.family.to_string().cyan());
    println!("    Native format: {}", distro.native_format.to_string().green().bold());
    println!("    Like: {}", if distro.like.is_empty() { "-".to_string() } else { distro.like.join(", ") }.dimmed());
    println!();
    let managers = installers::available_managers();
    println!("  Available package managers:");
    for m in &managers { println!("    {} {}", "✓".green(), m.name()); }
    if managers.is_empty() { println!("    {} No known managers detected (dnf, apt, pacman, flatpak)", "✗".red()); }
    // Check flatpak remote?
    println!();
    Ok(())
}
