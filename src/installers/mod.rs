use std::process::{Command, Stdio};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManagerKind {
    Dnf,
    Apt,
    Pacman,
    Zypper,
    Apk,
    Flatpak,
    Snap,
    Brew,
}

impl std::fmt::Display for ManagerKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ManagerKind::Dnf => "dnf",
            ManagerKind::Apt => "apt",
            ManagerKind::Pacman => "pacman",
            ManagerKind::Zypper => "zypper",
            ManagerKind::Apk => "apk",
            ManagerKind::Flatpak => "flatpak",
            ManagerKind::Snap => "snap",
            ManagerKind::Brew => "brew",
        };
        write!(f, "{}", s)
    }
}

pub trait PackageManager: Send + Sync {
    fn kind(&self) -> ManagerKind;
    fn is_available(&self) -> bool;
    fn detect(&self) -> bool { self.is_available() }
    fn install(&self, package_path: &str, yes: bool) -> anyhow::Result<()>;
    fn remove(&self, package_name: &str, yes: bool) -> anyhow::Result<()>;
    fn update(&self) -> anyhow::Result<()>;
    fn name(&self) -> &'static str;
}

fn which_exists(cmd: &str) -> bool {
    Command::new("which").arg(cmd).output().map(|o| o.status.success()).unwrap_or(false)
}

fn run_command(cmd: &str, args: &[&str], dry_run: bool) -> anyhow::Result<()> {
    if dry_run {
        println!("  [dry-run] {} {}", cmd, args.join(" "));
        return Ok(());
    }
    println!("  Running: {} {}", cmd, args.join(" "));
    let status = Command::new(cmd).args(args).stdin(Stdio::inherit()).stdout(Stdio::inherit()).stderr(Stdio::inherit()).status()?;
    if !status.success() {
        anyhow::bail!("Command {} failed with status {:?}", cmd, status.code());
    }
    Ok(())
}

// DNF Adapter
pub struct DnfAdapter { pub dry_run: bool }
impl DnfAdapter { pub fn new(dry_run: bool) -> Self { Self { dry_run } } }
impl PackageManager for DnfAdapter {
    fn kind(&self) -> ManagerKind { ManagerKind::Dnf }
    fn name(&self) -> &'static str { "dnf" }
    fn is_available(&self) -> bool { which_exists("dnf") || which_exists("dnf5") }
    fn install(&self, package_path: &str, yes: bool) -> anyhow::Result<()> {
        let dnf_cmd = if which_exists("dnf5") { "dnf5" } else { "dnf" };
        let mut args = vec!["install", "-y", package_path];
        if !yes { args = vec!["install", package_path]; }
        // For local rpm, dnf install <file> works
        run_command(dnf_cmd, &args, self.dry_run)
    }
    fn remove(&self, package_name: &str, yes: bool) -> anyhow::Result<()> {
        let dnf_cmd = if which_exists("dnf5") { "dnf5" } else { "dnf" };
        if yes {
            run_command(dnf_cmd, &["remove", "-y", package_name], self.dry_run)
        } else {
            run_command(dnf_cmd, &["remove", package_name], self.dry_run)
        }
    }
    fn update(&self) -> anyhow::Result<()> {
        let dnf_cmd = if which_exists("dnf5") { "dnf5" } else { "dnf" };
        run_command(dnf_cmd, &["upgrade", "-y"], self.dry_run)
    }
}

// APT Adapter
pub struct AptAdapter { pub dry_run: bool }
impl AptAdapter { pub fn new(dry_run: bool) -> Self { Self { dry_run } } }
impl PackageManager for AptAdapter {
    fn kind(&self) -> ManagerKind { ManagerKind::Apt }
    fn name(&self) -> &'static str { "apt" }
    fn is_available(&self) -> bool { which_exists("apt-get") || which_exists("apt") }
    fn install(&self, package_path: &str, _yes: bool) -> anyhow::Result<()> {
        // dpkg -i for local deb, else apt install
        if package_path.ends_with(".deb") {
            // Use apt to handle dependencies: apt install ./package.deb
            let path_arg = if package_path.starts_with('/') || package_path.starts_with("./") {
                package_path.to_string()
            } else {
                format!("./{}", package_path)
            };
            run_command("apt-get", &["install", "-y", &path_arg], self.dry_run)?;
        } else {
            run_command("apt-get", &["install", "-y", package_path], self.dry_run)?;
        }
        Ok(())
    }
    fn remove(&self, package_name: &str, _yes: bool) -> anyhow::Result<()> {
        run_command("apt-get", &["remove", "-y", package_name], self.dry_run)
    }
    fn update(&self) -> anyhow::Result<()> {
        run_command("apt-get", &["update"], self.dry_run)?;
        run_command("apt-get", &["upgrade", "-y"], self.dry_run)
    }
}

// Pacman Adapter
pub struct PacmanAdapter { pub dry_run: bool }
impl PacmanAdapter { pub fn new(dry_run: bool) -> Self { Self { dry_run } } }
impl PackageManager for PacmanAdapter {
    fn kind(&self) -> ManagerKind { ManagerKind::Pacman }
    fn name(&self) -> &'static str { "pacman" }
    fn is_available(&self) -> bool { which_exists("pacman") }
    fn install(&self, package_path: &str, _yes: bool) -> anyhow::Result<()> {
        if package_path.ends_with(".pkg.tar.zst") || package_path.ends_with(".pkg.tar.xz") {
            run_command("pacman", &["-U", "--noconfirm", package_path], self.dry_run)
        } else {
            run_command("pacman", &["-S", "--noconfirm", package_path], self.dry_run)
        }
    }
    fn remove(&self, package_name: &str, _yes: bool) -> anyhow::Result<()> {
        run_command("pacman", &["-R", "--noconfirm", package_name], self.dry_run)
    }
    fn update(&self) -> anyhow::Result<()> {
        run_command("pacman", &["-Syu", "--noconfirm"], self.dry_run)
    }
}

// Flatpak Adapter
pub struct FlatpakAdapter { pub dry_run: bool }
impl FlatpakAdapter { pub fn new(dry_run: bool) -> Self { Self { dry_run } } }
impl PackageManager for FlatpakAdapter {
    fn kind(&self) -> ManagerKind { ManagerKind::Flatpak }
    fn name(&self) -> &'static str { "flatpak" }
    fn is_available(&self) -> bool { which_exists("flatpak") }
    fn install(&self, package_path: &str, yes: bool) -> anyhow::Result<()> {
        // package_path here is app ID like com.example.App
        // Use flatpak install flathub <app-id> -y
        if package_path.ends_with(".flatpak") || package_path.ends_with(".flatpakref") {
            run_command("flatpak", &["install", "-y", package_path], self.dry_run)
        } else {
            // Assume app ID
            if yes {
                run_command("flatpak", &["install", "-y", "flathub", package_path], self.dry_run)
            } else {
                run_command("flatpak", &["install", "flathub", package_path], self.dry_run)
            }
        }
    }
    fn remove(&self, package_name: &str, _yes: bool) -> anyhow::Result<()> {
        run_command("flatpak", &["uninstall", "-y", package_name], self.dry_run)
    }
    fn update(&self) -> anyhow::Result<()> {
        run_command("flatpak", &["update", "-y"], self.dry_run)
    }
}

// Snap Adapter
pub struct SnapAdapter { pub dry_run: bool }
impl SnapAdapter { pub fn new(dry_run: bool) -> Self { Self { dry_run } } }
impl PackageManager for SnapAdapter {
    fn kind(&self) -> ManagerKind { ManagerKind::Snap }
    fn name(&self) -> &'static str { "snap" }
    fn is_available(&self) -> bool { which_exists("snap") }
    fn install(&self, package_path: &str, _yes: bool) -> anyhow::Result<()> {
        if package_path.ends_with(".snap") {
            run_command("snap", &["install", "--dangerous", package_path], self.dry_run)
        } else {
            run_command("snap", &["install", package_path], self.dry_run)
        }
    }
    fn remove(&self, package_name: &str, _yes: bool) -> anyhow::Result<()> {
        run_command("snap", &["remove", package_name], self.dry_run)
    }
    fn update(&self) -> anyhow::Result<()> {
        run_command("snap", &["refresh"], self.dry_run)
    }
}

// ZypperAdapter
pub struct ZypperAdapter { pub dry_run: bool }
impl ZypperAdapter { pub fn new(dry_run: bool) -> Self { Self { dry_run } } }
impl PackageManager for ZypperAdapter {
    fn kind(&self) -> ManagerKind { ManagerKind::Zypper }
    fn name(&self) -> &'static str { "zypper" }
    fn is_available(&self) -> bool { which_exists("zypper") }
    fn install(&self, package_path: &str, _yes: bool) -> anyhow::Result<()> {
        run_command("zypper", &["install", "-y", package_path], self.dry_run)
    }
    fn remove(&self, package_name: &str, _yes: bool) -> anyhow::Result<()> {
        run_command("zypper", &["remove", "-y", package_name], self.dry_run)
    }
    fn update(&self) -> anyhow::Result<()> {
        run_command("zypper", &["update", "-y"], self.dry_run)
    }
}

pub fn detect_package_manager(distro: &crate::distro::Distribution) -> Option<Box<dyn PackageManager>> {
    // Prefer native manager for format
    match distro.native_format {
        crate::distro::NativeFormat::Rpm => {
            if which_exists("dnf") || which_exists("dnf5") { return Some(Box::new(DnfAdapter::new(false))); }
            if which_exists("zypper") { return Some(Box::new(ZypperAdapter::new(false))); }
            if which_exists("yum") { return Some(Box::new(DnfAdapter::new(false))); }
        },
        crate::distro::NativeFormat::Deb => return Some(Box::new(AptAdapter::new(false))),
        crate::distro::NativeFormat::Pacman => return Some(Box::new(PacmanAdapter::new(false))),
        crate::distro::NativeFormat::Apk => {
            // Apk adapter not fully implemented; use apk
            if which_exists("apk") { // fallback to generic? We'll use dnf adapter placeholder but need apk
                struct ApkAdapter { dry_run: bool }
                impl PackageManager for ApkAdapter {
                    fn kind(&self) -> ManagerKind { ManagerKind::Apk }
                    fn name(&self) -> &'static str { "apk" }
                    fn is_available(&self) -> bool { which_exists("apk") }
                    fn install(&self, p: &str, _: bool) -> anyhow::Result<()> { run_command("apk", &["add", p], self.dry_run) }
                    fn remove(&self, p: &str, _: bool) -> anyhow::Result<()> { run_command("apk", &["del", p], self.dry_run) }
                    fn update(&self) -> anyhow::Result<()> { run_command("apk", &["update"], self.dry_run) }
                }
                return Some(Box::new(ApkAdapter{ dry_run: false }));
            }
        },
        _ => {}
    }
    // fallback check all
    if which_exists("dnf") { return Some(Box::new(DnfAdapter::new(false))); }
    if which_exists("dnf5") { return Some(Box::new(DnfAdapter::new(false))); }
    if which_exists("apt-get") { return Some(Box::new(AptAdapter::new(false))); }
    if which_exists("pacman") { return Some(Box::new(PacmanAdapter::new(false))); }
    if which_exists("flatpak") { return Some(Box::new(FlatpakAdapter::new(false))); }
    None
}

pub fn available_managers() -> Vec<Box<dyn PackageManager>> {
    let mut v: Vec<Box<dyn PackageManager>> = Vec::new();
    let dnf = DnfAdapter::new(false);
    if dnf.is_available() { v.push(Box::new(dnf)); }
    let apt = AptAdapter::new(false);
    if apt.is_available() { v.push(Box::new(apt)); }
    let pacman = PacmanAdapter::new(false);
    if pacman.is_available() { v.push(Box::new(pacman)); }
    let flatpak = FlatpakAdapter::new(false);
    if flatpak.is_available() { v.push(Box::new(flatpak)); }
    let snap = SnapAdapter::new(false);
    if snap.is_available() { v.push(Box::new(snap)); }
    let zypper = ZypperAdapter::new(false);
    if zypper.is_available() { v.push(Box::new(zypper)); }
    v
}
