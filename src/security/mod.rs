use sha2::{Sha256, Digest};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub verified: bool,
    pub checksum_matched: Option<bool>,
    pub expected: Option<String>,
    pub actual: Option<String>,
    pub warnings: Vec<String>,
}

pub fn compute_sha256(path: &Path) -> anyhow::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    let result = hasher.finalize();
    Ok(hex::encode(result))
}

pub fn verify_checksum(path: &Path, expected: &str) -> anyhow::Result<bool> {
    let actual = compute_sha256(path)?;
    let exp = expected.trim().to_lowercase();
    // Expected may be in format "sha256: <hash>" or "<hash>  filename" or just hash
    let clean_exp = if exp.contains(' ') {
        exp.split_whitespace().next().unwrap_or(&exp).to_string()
    } else if exp.contains(':') {
        exp.split(':').last().unwrap_or(&exp).to_string()
    } else {
        exp
    };
    let clean_exp = clean_exp.split_whitespace().next().unwrap_or("").trim().to_string();
    // Also handle if expected is longer than 64 chars due to extra
    let exp_hash = if clean_exp.len() > 64 { clean_exp[..64].to_string() } else { clean_exp };
    Ok(actual.to_lowercase() == exp_hash.to_lowercase())
}

pub fn fetch_checksum_file(url: &str) -> Option<String> {
    // Attempt to download checksum file content
    // This is used when release assets include checksums.txt
    let resp = ureq::get(url).call().ok()?;
    if resp.status() >= 400 { return None; }
    let text = resp.into_string().ok()?;
    Some(text)
}

pub fn parse_checksums_file(content: &str, filename: &str) -> Option<String> {
    // checksums.txt typically format: "<hash>  <filename>" per line
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        // Split
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            // Check if filename matches
            let hash = parts[0];
            let file_part = parts[1].trim_start_matches('*');
            if file_part == filename || file_part.ends_with(filename) || filename.ends_with(file_part) {
                return Some(hash.to_string());
            }
        } else if parts.len() == 1 && line.len() >= 64 {
            // Single hash maybe for single file?
            // Only use if file count is 1 - ambiguous
            // We'll not guess
        }
    }
    None
}

pub fn verify_package(path: &Path, expected_checksum: Option<&str>, warnings: &mut Vec<String>) -> VerificationResult {
    let mut result = VerificationResult {
        verified: false,
        checksum_matched: None,
        expected: expected_checksum.map(|s| s.to_string()),
        actual: None,
        warnings: Vec::new(),
    };
    // Compute actual
    match compute_sha256(path) {
        Ok(actual) => {
            result.actual = Some(actual.clone());
            if let Some(exp) = expected_checksum {
                match verify_checksum(path, exp) {
                    Ok(matched) => {
                        result.checksum_matched = Some(matched);
                        result.verified = matched;
                        if !matched {
                            warnings.push(format!("Checksum mismatch! Expected {}, got {}", exp, actual));
                            result.warnings.push("Checksum mismatch".to_string());
                        }
                    },
                    Err(e) => {
                        warnings.push(format!("Failed to verify checksum: {}", e));
                        result.warnings.push(format!("verify error: {}", e));
                    }
                }
            } else {
                warnings.push("No checksum provided — skipping verification (use --verify or ensure upstream provides checksums)".to_string());
                result.verified = true; // no checksum to verify, but we consider file present
                result.warnings.push("no checksum available".to_string());
            }
        },
        Err(e) => {
            warnings.push(format!("Failed to compute checksum: {}", e));
            result.warnings.push(format!("hash error: {}", e));
        }
    }
    result
}

pub fn validate_package_format(path: &Path, expected_format: &crate::packages::PackageFormat) -> anyhow::Result<()> {
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let detected = crate::packages::PackageFormat::from_filename(filename);
    if detected != *expected_format && detected != crate::packages::PackageFormat::Unknown {
        anyhow::bail!("Package format mismatch: expected {}, detected {} for {}", expected_format, detected, filename);
    }
    // Additional check: file magic? For now filename suffices
    if !path.exists() {
        anyhow::bail!("Package file does not exist: {:?}", path);
    }
    // Check file size >0
    let meta = std::fs::metadata(path)?;
    if meta.len() == 0 {
        anyhow::bail!("Package file is empty: {:?}", path);
    }
    Ok(())
}

pub fn prompt_confirmation(message: &str, auto_yes: bool) -> bool {
    if auto_yes {
        return true;
    }
    use std::io::{self, Write};
    print!("{} [Y/n]: ", message);
    io::stdout().flush().ok();
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        let input = input.trim().to_lowercase();
        input.is_empty() || input == "y" || input == "yes"
    } else {
        false
    }
}

pub fn security_summary_for_package(asset: &crate::packages::PackageAsset, distro: &crate::distro::Distribution) -> Vec<String> {
    let mut warnings = Vec::new();
    // Warn about suspicious formats
    match asset.format {
        crate::packages::PackageFormat::TarGz | crate::packages::PackageFormat::Zip | crate::packages::PackageFormat::Binary | crate::packages::PackageFormat::Unknown => {
            warnings.push(format!("Package format '{}' may require manual installation or script execution. Vista will not run arbitrary install scripts automatically.", asset.format));
        },
        _ => {}
    }
    if asset.arch.is_none() {
        warnings.push("Architecture not detected in filename — please verify compatibility.".to_string());
    } else if let Some(arch) = &asset.arch {
        if arch != "all" && !crate::packages::is_arch_compatible(&Some(arch.clone()), &distro.architecture) {
            warnings.push(format!("Architecture mismatch: package is {} but system is {}", arch, distro.architecture));
        }
    }
    warnings
}
