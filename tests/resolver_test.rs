use vista::distro::{Distribution, Family, NativeFormat};
use vista::packages::{PackageAsset, score_asset};
use vista::github::GithubRelease;

fn fedora_distro() -> Distribution {
    Distribution {
        id: "fedora".to_string(),
        name: "Fedora Linux 44".to_string(),
        version: "44 (KDE)".to_string(),
        version_id: "44".to_string(),
        architecture: "x86_64".to_string(),
        family: Family::Rpm,
        native_format: NativeFormat::Rpm,
        like: vec![],
        codename: "".to_string(),
    }
}

fn debian_distro() -> Distribution {
    Distribution {
        id: "debian".to_string(),
        name: "Debian GNU/Linux 12".to_string(),
        version: "12 (bookworm)".to_string(),
        version_id: "12".to_string(),
        architecture: "x86_64".to_string(),
        family: Family::Deb,
        native_format: NativeFormat::Deb,
        like: vec![],
        codename: "bookworm".to_string(),
    }
}

fn load_mixed_release() -> GithubRelease {
    let data = std::fs::read_to_string("tests/fixtures/github-release-mixed.json").unwrap();
    serde_json::from_str(&data).unwrap()
}

#[test]
fn fedora_prefers_rpm_over_deb() {
    let distro = fedora_distro();
    let release = load_mixed_release();
    let assets: Vec<PackageAsset> = release
        .assets
        .iter()
        .map(|a| PackageAsset::from_github_asset(a.name.clone(), a.browser_download_url.clone(), Some(a.size)))
        .collect();
    let scored: Vec<_> = assets.iter().map(|a| score_asset(a, &distro, true)).collect();
    let best = scored.iter().filter(|s| s.is_compatible).max_by_key(|s| s.score).unwrap();
    assert_eq!(best.asset.filename, "cool-app-2.4.1-x86_64.rpm", "Fedora must pick rpm: {:?}", scored);
    // deb must be incompatible on rpm system
    let deb = scored.iter().find(|s| s.asset.filename.ends_with(".deb")).unwrap();
    assert!(!deb.is_compatible, "deb must be rejected on Fedora");
    // checksum must be rejected
    let chk = scored.iter().find(|s| s.asset.filename == "checksums.txt").unwrap();
    assert!(!chk.is_compatible);
    assert_eq!(chk.score, -1000);
}

#[test]
fn debian_prefers_deb_over_rpm() {
    let distro = debian_distro();
    let release = load_mixed_release();
    let assets: Vec<PackageAsset> = release
        .assets
        .iter()
        .map(|a| PackageAsset::from_github_asset(a.name.clone(), a.browser_download_url.clone(), Some(a.size)))
        .collect();
    let scored: Vec<_> = assets.iter().map(|a| score_asset(a, &distro, true)).collect();
    let best = scored.iter().filter(|s| s.is_compatible).max_by_key(|s| s.score).unwrap();
    assert_eq!(best.asset.filename, "cool-app-2.4.1-amd64.deb", "Debian must pick deb");
}

#[test]
fn wrong_arch_rejected() {
    let distro = fedora_distro();
    let release = load_mixed_release();
    let assets: Vec<PackageAsset> = release
        .assets
        .iter()
        .map(|a| PackageAsset::from_github_asset(a.name.clone(), a.browser_download_url.clone(), Some(a.size)))
        .collect();
    let scored: Vec<_> = assets.iter().map(|a| score_asset(a, &distro, true)).collect();
    let aarch = scored.iter().find(|s| s.asset.filename.contains("aarch64")).unwrap();
    assert!(!aarch.is_compatible, "aarch64 rpm must be rejected on x86_64");
}

#[test]
fn no_native_triggers_fallback() {
    let distro = fedora_distro();
    let data = std::fs::read_to_string("tests/fixtures/github-release-no-native.json").unwrap();
    let release: GithubRelease = serde_json::from_str(&data).unwrap();
    let assets: Vec<PackageAsset> = release
        .assets
        .iter()
        .map(|a| PackageAsset::from_github_asset(a.name.clone(), a.browser_download_url.clone(), Some(a.size)))
        .collect();
    let scored: Vec<_> = assets.iter().map(|a| score_asset(a, &distro, true)).collect();
    let compatible: Vec<_> = scored.iter().filter(|s| s.is_compatible && s.score > 0).collect();
    assert!(compatible.is_empty(), "no compatible native package expected, got {:?}", compatible);
    // Flathub fixture must exist as fallback
    let flathub: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string("tests/fixtures/flathub-search.json").unwrap()).unwrap();
    assert!(flathub.as_array().unwrap().len() >= 1);
}
