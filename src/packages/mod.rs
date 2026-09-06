use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum PackageFormat {
    Rpm,
    Deb,
    Pacman, // .pkg.tar.zst, .pkg.tar.xz, .pkg.tar.gz
    Apk,
    AppImage,
    Flatpak,
    FlatpakRef,
    Snap,
    TarGz,
    TarXz,
    TarBz2,
    Zip,
    Binary,
    Unknown,
}

impl std::fmt::Display for PackageFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            PackageFormat::Rpm => "rpm",
            PackageFormat::Deb => "deb",
            PackageFormat::Pacman => "pacman",
            PackageFormat::Apk => "apk",
            PackageFormat::AppImage => "AppImage",
            PackageFormat::Flatpak => "flatpak",
            PackageFormat::FlatpakRef => "flatpakref",
            PackageFormat::Snap => "snap",
            PackageFormat::TarGz => "tar.gz",
            PackageFormat::TarXz => "tar.xz",
            PackageFormat::TarBz2 => "tar.bz2",
            PackageFormat::Zip => "zip",
            PackageFormat::Binary => "binary",
            PackageFormat::Unknown => "unknown",
        };
        write!(f, "{}", s)
    }
}

impl PackageFormat {
    pub fn from_filename(filename: &str) -> Self {
        let lower = filename.to_lowercase();
        if lower.ends_with(".rpm") { PackageFormat::Rpm }
        else if lower.ends_with(".deb") { PackageFormat::Deb }
        else if lower.ends_with(".pkg.tar.zst") || lower.ends_with(".pkg.tar.xz") || lower.ends_with(".pkg.tar.gz") {
            PackageFormat::Pacman
        }
        else if lower.ends_with(".apk") { PackageFormat::Apk }
        else if lower.ends_with(".appimage") { PackageFormat::AppImage }
        else if lower.ends_with(".flatpak") { PackageFormat::Flatpak }
        else if lower.ends_with(".flatpakref") { PackageFormat::FlatpakRef }
        else if lower.ends_with(".snap") { PackageFormat::Snap }
        else if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") { PackageFormat::TarGz }
        else if lower.ends_with(".tar.xz") || lower.ends_with(".txz") { PackageFormat::TarXz }
        else if lower.ends_with(".tar.bz2") || lower.ends_with(".tbz2") { PackageFormat::TarBz2 }
        else if lower.ends_with(".zip") { PackageFormat::Zip }
        else { PackageFormat::Unknown }
    }

    pub fn is_native_for(&self, native: &crate::distro::NativeFormat) -> bool {
        match (self, native) {
            (PackageFormat::Rpm, crate::distro::NativeFormat::Rpm) => true,
            (PackageFormat::Deb, crate::distro::NativeFormat::Deb) => true,
            (PackageFormat::Pacman, crate::distro::NativeFormat::Pacman) => true,
            (PackageFormat::Apk, crate::distro::NativeFormat::Apk) => true,
            (PackageFormat::Snap, crate::distro::NativeFormat::Snap) => true,
            _ => false,
        }
    }

    pub fn priority_score(&self) -> i32 {
        match self {
            PackageFormat::Rpm => 100,
            PackageFormat::Deb => 100,
            PackageFormat::Pacman => 100,
            PackageFormat::Apk => 100,
            PackageFormat::Snap => 80,
            PackageFormat::AppImage => 10,
            PackageFormat::Flatpak => 5,
            PackageFormat::FlatpakRef => 5,
            PackageFormat::TarGz => 2,
            PackageFormat::TarXz => 2,
            _ => 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageAsset {
    pub filename: String,
    pub url: String,
    pub size: Option<u64>,
    pub format: PackageFormat,
    pub arch: Option<String>,
    pub version: Option<String>,
}

impl PackageAsset {
    pub fn from_github_asset(name: String, url: String, size: Option<u64>) -> Self {
        let format = PackageFormat::from_filename(&name);
        let arch = detect_arch_from_filename(&name);
        let version = extract_version_from_filename(&name);
        Self { filename: name, url, size, format, arch, version }
    }
}

/// Normalize architecture strings found in filenames
pub fn normalize_arch_label(raw: &str) -> Option<String> {
    let lower = raw.to_lowercase();
    match lower.as_str() {
        "x86_64" | "amd64" | "x64" | "x86-64" | "x86_64_v3" | "intel64" => Some("x86_64".to_string()),
        "aarch64" | "arm64" | "arm64v8" | "arm64_v8" | "arm8" | "arm_64" => Some("aarch64".to_string()),
        "armv7l" | "armv7" | "armhf" | "arm" => Some("armv7l".to_string()),
        "i386" | "i486" | "i586" | "i686" | "x86" | "386" => Some("i386".to_string()),
        "ppc64le" | "ppc64el" => Some("ppc64le".to_string()),
        "s390x" => Some("s390x".to_string()),
        "riscv64" => Some("riscv64".to_string()),
        "all" | "any" | "noarch" | "universal" => Some("all".to_string()),
        _ => None,
    }
}

/// Detect arch from filename by scanning for known arch tokens
pub fn detect_arch_from_filename(filename: &str) -> Option<String> {
    let lower = filename.to_lowercase();
    // Split by non-alphanumeric delimiters
    // Tokens like x86_64, amd64, aarch64, arm64, etc.
    // Use regex to find arch patterns
    let patterns = [
        ("x86_64", "x86_64"),
        ("amd64", "x86_64"),
        ("x64", "x86_64"),
        ("aarch64", "aarch64"),
        ("arm64", "aarch64"),
        ("armv7l", "armv7l"),
        ("armv7", "armv7l"),
        ("armhf", "armv7l"),
        ("i386", "i386"),
        ("i686", "i386"),
        ("ppc64le", "ppc64le"),
        ("s390x", "s390x"),
        ("riscv64", "riscv64"),
        ("all", "all"),
        ("noarch", "all"),
        ("universal", "all"),
    ];

    for (pat, normalized) in patterns {
        // Check if filename contains pat as separate token or part
        // Use simple contains for now, but try to be precise: split by . - _ 
        if lower.contains(pat) {
            // extra check: ensure it's not substring of other word accidentally
            // For now, accept.
            return Some(normalized.to_string());
        }
    }
    None
}

pub fn extract_version_from_filename(filename: &str) -> Option<String> {
    // Use regex to find version like v1.2.3 or 1.2.3
    // Look for pattern: -1.2.3 , _1.2.3, v1.2.3
    let re = Regex::new(r"[vV]?(\d+\.\d+(?:\.\d+)?(?:[-_\.][a-zA-Z0-9]+)*)").ok()?;
    let caps = re.captures(filename)?;
    caps.get(1).map(|m| m.as_str().to_string())
}

/// Check if asset arch is compatible with system arch
pub fn is_arch_compatible(asset_arch: &Option<String>, system_arch: &str) -> bool {
    match asset_arch {
        None => true, // unknown arch, assume compatible? but lower score
        Some(a) => {
            if a == "all" { return true; }
            let sys = crate::distro::normalize_arch(system_arch);
            let asset_norm = crate::distro::normalize_arch(a);
            sys == asset_norm
        }
    }
}

/// Determine if asset is checksum file (not installable)
pub fn is_checksum_file(filename: &str) -> bool {
    let lower = filename.to_lowercase();
    lower.contains("checksum") || lower.contains("sha256") || lower.contains("sha512")
        || lower == "checksums.txt" || lower.ends_with(".sha256") || lower.ends_with(".sha512")
        || lower.ends_with(".sha256sum") || lower.ends_with(".md5") || lower.contains("hash")
}

/// Determine if asset is signature file
pub fn is_signature_file(filename: &str) -> bool {
    let lower = filename.to_lowercase();
    lower.ends_with(".asc") || lower.ends_with(".sig") || lower.ends_with(".gpg")
}

/// Determine if asset is source code archive
pub fn is_source_archive(filename: &str) -> bool {
    let lower = filename.to_lowercase();
    (lower.contains("source") && (lower.ends_with(".tar.gz") || lower.ends_with(".zip")))
        || lower == "source.tar.gz" || lower == "source.zip"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredAsset {
    pub asset: PackageAsset,
    pub score: i32,
    pub reasons: Vec<String>,
    pub is_compatible: bool,
}

pub fn score_asset(asset: &PackageAsset, distro: &crate::distro::Distribution, is_stable: bool) -> ScoredAsset {
    let mut score: i32 = 0;
    let mut reasons = Vec::new();
    let mut compatible = true;

    // Check if checksum/signature/source — these are not installable
    if is_checksum_file(&asset.filename) {
        return ScoredAsset { asset: asset.clone(), score: -1000, reasons: vec!["checksum file".to_string()], is_compatible: false };
    }
    if is_signature_file(&asset.filename) {
        return ScoredAsset { asset: asset.clone(), score: -1000, reasons: vec!["signature file".to_string()], is_compatible: false };
    }
    if is_source_archive(&asset.filename) {
        return ScoredAsset { asset: asset.clone(), score: -1000, reasons: vec!["source archive".to_string()], is_compatible: false };
    }

    // Native package +100
    if asset.format.is_native_for(&distro.native_format) {
        score += 100;
        reasons.push(format!("native package ({}) +100", asset.format));
    } else {
        // Check if format is incompatible
        match (&asset.format, &distro.native_format) {
            (PackageFormat::Rpm, crate::distro::NativeFormat::Deb) => {
                score -= 50;
                reasons.push("incompatible: rpm on deb system -50".to_string());
                compatible = false;
            },
            (PackageFormat::Deb, crate::distro::NativeFormat::Rpm) => {
                score -= 50;
                reasons.push("incompatible: deb on rpm system -50".to_string());
                compatible = false;
            },
            (PackageFormat::Deb, crate::distro::NativeFormat::Pacman) => {
                score -= 50;
                reasons.push("incompatible: deb on arch -50".to_string());
                compatible = false;
            },
            (PackageFormat::Rpm, crate::distro::NativeFormat::Pacman) => {
                score -= 50;
                reasons.push("incompatible: rpm on arch -50".to_string());
                compatible = false;
            },
            (PackageFormat::AppImage, _) => {
                score += 10;
                reasons.push("AppImage +10".to_string());
            },
            (PackageFormat::Flatpak, _) | (PackageFormat::FlatpakRef, _) => {
                score += 5;
                reasons.push("Flatpak +5".to_string());
            },
            (PackageFormat::Unknown, _) => {
                score -= 20;
                reasons.push("unknown format -20".to_string());
                compatible = false;
            },
            _ => {
                score += asset.format.priority_score();
                if asset.format.priority_score() > 0 {
                    reasons.push(format!("format {} +{}", asset.format, asset.format.priority_score()));
                }
            }
        }
    }

    // Correct architecture +50
    if let Some(arch) = &asset.arch {
        if arch == "all" {
            score += 30;
            reasons.push("universal arch +30".to_string());
        } else if is_arch_compatible(&Some(arch.clone()), &distro.architecture) {
            score += 50;
            reasons.push(format!("correct arch {} +50", arch));
        } else {
            score -= 100;
            reasons.push(format!("wrong arch {} -100", arch));
            compatible = false;
        }
    } else {
        // No arch detected - slight penalty
        score += 5;
        reasons.push("unknown arch +5 (assumed compatible)".to_string());
    }

    // Check distro-specific naming hints? e.g., fedora vs ubuntu vs debian vs arch
    let lower = asset.filename.to_lowercase();
    if lower.contains(&distro.id) {
        score += 50;
        reasons.push(format!("distro match {} +50", distro.id));
    } else if distro.like.iter().any(|l| lower.contains(&l.to_lowercase())) {
        score += 30;
        reasons.push("distro family match +30".to_string());
    }
    // If asset mentions a different distro explicitly, penalize
    // Detect common distro markers
    let other_distros = ["fedora", "ubuntu", "debian", "arch", "centos", "rhel", "opensuse", "alpine"];
    for od in other_distros {
        if od != distro.id && lower.contains(od) && !distro.like.contains(&od.to_string()) {
            // e.g., ubuntu asset on fedora
            if (od == "ubuntu" || od == "debian") && distro.family == crate::distro::Family::Rpm {
                score -= 20;
                reasons.push(format!("mismatched distro hint {} -20", od));
            }
            if (od == "fedora" || od == "rhel" || od == "centos") && distro.family == crate::distro::Family::Deb {
                score -= 20;
                reasons.push(format!("mismatched distro hint {} -20", od));
            }
        }
    }

    if is_stable {
        score += 25;
        reasons.push("stable release +25".to_string());
    } else {
        score -= 10;
        reasons.push("prerelease -10".to_string());
    }

    // GitHub release +20 (if applicable — we assume all assets here are from GitHub)
    score += 20;
    reasons.push("GitHub release +20".to_string());

    ScoredAsset { asset: asset.clone(), score, reasons, is_compatible: compatible }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_format_detection() {
        assert_eq!(PackageFormat::from_filename("app-1.0.rpm"), PackageFormat::Rpm);
        assert_eq!(PackageFormat::from_filename("app_1.0_amd64.deb"), PackageFormat::Deb);
        assert_eq!(PackageFormat::from_filename("app.AppImage"), PackageFormat::AppImage);
        assert_eq!(PackageFormat::from_filename("app.pkg.tar.zst"), PackageFormat::Pacman);
    }
    #[test]
    fn test_arch_detection() {
        assert_eq!(detect_arch_from_filename("app-1.0-x86_64.rpm"), Some("x86_64".to_string()));
        assert_eq!(detect_arch_from_filename("app-1.0-amd64.deb"), Some("x86_64".to_string()));
        assert_eq!(detect_arch_from_filename("app-1.0-aarch64.rpm"), Some("aarch64".to_string()));
        assert_eq!(detect_arch_from_filename("app-1.0-arm64.deb"), Some("aarch64".to_string()));
    }
    #[test]
    fn test_is_checksum() {
        assert!(is_checksum_file("checksums.txt"));
        assert!(is_checksum_file("app-1.0.sha256"));
        assert!(!is_checksum_file("app-1.0.rpm"));
    }
}
