use std::collections::HashMap;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Family {
    Rpm,
    Deb,
    Arch,
    Alpine,
    Snap,
    Unknown,
}

impl std::fmt::Display for Family {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Family::Rpm => write!(f, "rpm"),
            Family::Deb => write!(f, "deb"),
            Family::Arch => write!(f, "arch"),
            Family::Alpine => write!(f, "alpine"),
            Family::Snap => write!(f, "snap"),
            Family::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NativeFormat {
    Rpm,
    Deb,
    Pacman,
    Apk,
    Snap,
    Unknown,
}

impl std::fmt::Display for NativeFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NativeFormat::Rpm => write!(f, "rpm"),
            NativeFormat::Deb => write!(f, "deb"),
            NativeFormat::Pacman => write!(f, "pacman"),
            NativeFormat::Apk => write!(f, "apk"),
            NativeFormat::Snap => write!(f, "snap"),
            NativeFormat::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Distribution {
    pub id: String,
    pub name: String,
    pub version: String,
    pub version_id: String,
    pub architecture: String,
    pub family: Family,
    pub native_format: NativeFormat,
    pub like: Vec<String>,
    pub codename: String,
}

impl std::fmt::Display for Distribution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {} ({}, {})", self.name, self.version_id, self.architecture, self.family, self.native_format)
    }
}

impl Distribution {
    pub fn detect() -> Self {
        detect_distribution()
    }

    pub fn is_rpm(&self) -> bool { self.family == Family::Rpm }
    pub fn is_deb(&self) -> bool { self.family == Family::Deb }
    pub fn is_arch(&self) -> bool { self.family == Family::Arch }
}

pub fn normalize_arch(raw: &str) -> String {
    let lower = raw.to_lowercase();
    match lower.as_str() {
        "x86_64" | "amd64" | "x64" | "x86-64" | "x86_64_v3" => "x86_64".to_string(),
        "aarch64" | "arm64" | "arm64v8" | "arm64_v8" | "arm8" => "aarch64".to_string(),
        "armv7l" | "armv7" | "armhf" | "armv7hl" => "armv7l".to_string(),
        "armv6l" | "armv6" => "armv6l".to_string(),
        "i386" | "i486" | "i586" | "i686" | "x86" | "386" => "i386".to_string(),
        "ppc64le" | "ppc64el" => "ppc64le".to_string(),
        "ppc64" => "ppc64".to_string(),
        "s390x" => "s390x".to_string(),
        "riscv64" => "riscv64".to_string(),
        other => other.to_string(),
    }
}

pub fn detect_arch() -> String {
    // Try uname -m
    let uname_arch = Command::new("uname")
        .arg("-m")
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else { None }
        })
        .unwrap_or_else(|| std::env::consts::ARCH.to_string());
    normalize_arch(&uname_arch)
}

fn parse_os_release() -> HashMap<String, String> {
    let mut map = HashMap::new();
    let content = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        if let Some(eq) = line.find('=') {
            let key = line[..eq].trim().to_string();
            let mut val = line[eq+1..].trim().to_string();
            // Remove surrounding quotes
            if (val.starts_with('"') && val.ends_with('"')) || (val.starts_with('\'') && val.ends_with('\'')) {
                val = val[1..val.len()-1].to_string();
            }
            map.insert(key, val);
        }
    }
    // Fallback to /usr/lib/os-release
    if map.is_empty() {
        if let Ok(content) = std::fs::read_to_string("/usr/lib/os-release") {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') { continue; }
                if let Some(eq) = line.find('=') {
                    let key = line[..eq].trim().to_string();
                    let mut val = line[eq+1..].trim().to_string();
                    if (val.starts_with('"') && val.ends_with('"')) || (val.starts_with('\'') && val.ends_with('\'')) {
                        val = val[1..val.len()-1].to_string();
                    }
                    map.insert(key, val);
                }
            }
        }
    }
    map
}

fn detect_family(id: &str, like: &[String]) -> (Family, NativeFormat) {
    let id_lower = id.to_lowercase();
    let like_lower: Vec<String> = like.iter().map(|s| s.to_lowercase()).collect();

    let is_like = |target: &str| like_lower.iter().any(|l| l == target);

    match id_lower.as_str() {
        "fedora" | "rhel" | "centos" | "almalinux" | "rocky" | "ol" | "oracle" | "amzn" | "amazon" => {
            (Family::Rpm, NativeFormat::Rpm)
        },
        "opensuse" | "opensuse-leap" | "opensuse-tumbleweed" | "sles" | "suse" => {
            (Family::Rpm, NativeFormat::Rpm)
        },
        "mageia" | "fedora" | "rhel" => (Family::Rpm, NativeFormat::Rpm),
        "debian" | "raspbian" => (Family::Deb, NativeFormat::Deb),
        "ubuntu" | "linuxmint" | "mint" | "pop" | "elementary" | "kali" | "zorin" | "neon" => {
            (Family::Deb, NativeFormat::Deb)
        },
        "arch" | "manjaro" | "endeavouros" | "garuda" | "artix" | "archarm" => {
            (Family::Arch, NativeFormat::Pacman)
        },
        "alpine" => (Family::Alpine, NativeFormat::Apk),
        _ => {
            // Check ID_LIKE
            if is_like("fedora") || is_like("rhel") || is_like("centos") || is_like("suse") || is_like("opensuse") {
                (Family::Rpm, NativeFormat::Rpm)
            } else if is_like("debian") || is_like("ubuntu") {
                (Family::Deb, NativeFormat::Deb)
            } else if is_like("arch") {
                (Family::Arch, NativeFormat::Pacman)
            } else if is_like("alpine") {
                (Family::Alpine, NativeFormat::Apk)
            } else {
                // Fallback: check available package managers
                if which_exists("dnf") || which_exists("yum") || which_exists("rpm") {
                    (Family::Rpm, NativeFormat::Rpm)
                } else if which_exists("apt") || which_exists("apt-get") || which_exists("dpkg") {
                    (Family::Deb, NativeFormat::Deb)
                } else if which_exists("pacman") {
                    (Family::Arch, NativeFormat::Pacman)
                } else if which_exists("apk") {
                    (Family::Alpine, NativeFormat::Apk)
                } else {
                    (Family::Unknown, NativeFormat::Unknown)
                }
            }
        }
    }
}

fn which_exists(cmd: &str) -> bool {
    Command::new("which").arg(cmd).output().map(|o| o.status.success()).unwrap_or(false)
}

pub fn detect_distribution() -> Distribution {
    let os = parse_os_release();
    let id = os.get("ID").cloned().unwrap_or_else(|| "unknown".to_string());
    let name = os.get("PRETTY_NAME").cloned()
        .or_else(|| os.get("NAME").cloned())
        .unwrap_or_else(|| id.clone());
    let version = os.get("VERSION").cloned().unwrap_or_default();
    let version_id = os.get("VERSION_ID").cloned().unwrap_or_default();
    let codename = os.get("VERSION_CODENAME").cloned()
        .or_else(|| os.get("UBUNTU_CODENAME").cloned())
        .unwrap_or_default();
    let like_raw = os.get("ID_LIKE").cloned().unwrap_or_default();
    let like: Vec<String> = like_raw.split_whitespace().map(|s| s.to_string()).collect();
    let arch = detect_arch();
    let (family, native_format) = detect_family(&id, &like);

    Distribution {
        id: id.to_lowercase(),
        name,
        version,
        version_id,
        architecture: arch,
        family,
        native_format,
        like,
        codename,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_normalize_arch() {
        assert_eq!(normalize_arch("amd64"), "x86_64");
        assert_eq!(normalize_arch("x86_64"), "x86_64");
        assert_eq!(normalize_arch("arm64"), "aarch64");
        assert_eq!(normalize_arch("aarch64"), "aarch64");
        assert_eq!(normalize_arch("i386"), "i386");
    }
    #[test]
    fn test_detect_arch_not_empty() {
        let a = detect_arch();
        assert!(!a.is_empty());
    }
    #[test]
    fn test_detect_distro() {
        let d = detect_distribution();
        assert!(!d.id.is_empty());
        assert!(!d.architecture.is_empty());
    }
}
