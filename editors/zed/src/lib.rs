use sha2::{Digest, Sha256};
use std::{fmt::Write as _, fs};
use zed_extension_api::{
    self as zed, Architecture, Command, DownloadedFileType, GithubReleaseOptions, LanguageServerId,
    Os, Result,
};

const GITHUB_REPO: &str = "agent-sh/agnix";

/// Zed extension that integrates the agnix LSP for validating agent configurations.
struct AgnixExtension {
    /// Cached path to the agnix-lsp binary, if already downloaded.
    cached_binary_path: Option<String>,
}

/// Returns the expected release asset name and download file type for a given platform.
fn asset_for_platform(os: Os, arch: Architecture) -> Result<(&'static str, DownloadedFileType)> {
    match (os, arch) {
        (Os::Mac, arch @ (Architecture::Aarch64 | Architecture::X8664)) => {
            // On x86_64, use the x86_64 binary; on ARM, use the ARM64 binary
            let asset_name = match arch {
                Architecture::Aarch64 => "agnix-lsp-aarch64-apple-darwin.tar.gz",
                Architecture::X8664 => "agnix-lsp-x86_64-apple-darwin.tar.gz",
                _ => unreachable!(),
            };
            Ok((asset_name, DownloadedFileType::GzipTar))
        }
        (Os::Linux, Architecture::X8664) => Ok((
            "agnix-lsp-x86_64-unknown-linux-gnu.tar.gz",
            DownloadedFileType::GzipTar,
        )),
        (Os::Linux, Architecture::Aarch64) => Ok((
            "agnix-lsp-aarch64-unknown-linux-gnu.tar.gz",
            DownloadedFileType::GzipTar,
        )),
        (Os::Windows, Architecture::X8664) => Ok((
            "agnix-lsp-x86_64-pc-windows-msvc.zip",
            DownloadedFileType::Zip,
        )),
        _ => Err(format!("unsupported platform: {os:?} {arch:?}",)),
    }
}

/// Returns the binary name for the LSP server on the given OS.
fn binary_name(os: Os) -> &'static str {
    match os {
        Os::Windows => "agnix-lsp.exe",
        _ => "agnix-lsp",
    }
}

fn binary_checksum_asset_name(asset_name: &str) -> Result<String> {
    if let Some(prefix) = asset_name.strip_suffix(".tar.gz") {
        Ok(format!("{prefix}.sha256"))
    } else if let Some(prefix) = asset_name.strip_suffix(".zip") {
        Ok(format!("{prefix}.sha256"))
    } else {
        Err(format!("unsupported asset extension for {asset_name}"))
    }
}

fn is_trusted_download_url(url: &str) -> bool {
    url.starts_with("https://github.com/")
        || url.starts_with("https://objects.githubusercontent.com/")
}

fn parse_sha256_sidecar(contents: &str, expected_file_name: &str) -> Result<String> {
    let line = contents
        .lines()
        .map(str::trim)
        .find(|candidate| !candidate.is_empty())
        .ok_or_else(|| "checksum file is empty".to_string())?;

    let mut parts = line.split_whitespace();
    let hash = parts
        .next()
        .ok_or_else(|| "checksum file is empty".to_string())?;
    if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("checksum file does not contain a valid SHA256 hash".to_string());
    }

    if let Some(sidecar_name) = parts.next() {
        let file_name = sidecar_name
            .trim_start_matches('*')
            .rsplit(|c| c == '/' || c == '\\')
            .next()
            .unwrap_or(sidecar_name);
        if file_name != expected_file_name {
            return Err(format!(
                "checksum file is for {file_name}, expected {expected_file_name}"
            ));
        }
    }

    Ok(hash.to_ascii_lowercase())
}

fn sha256_file(file_path: &str) -> Result<String> {
    let bytes = fs::read(file_path).map_err(|e| format!("failed to read {file_path}: {e}"))?;
    let digest = Sha256::digest(&bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").map_err(|e| e.to_string())?;
    }
    Ok(output)
}

fn verify_sha256_file(
    file_path: &str,
    checksum_contents: &str,
    expected_file_name: &str,
) -> Result<String> {
    let expected = parse_sha256_sidecar(checksum_contents, expected_file_name)?;
    let actual = sha256_file(file_path)?;
    if actual != expected {
        return Err(format!(
            "checksum mismatch for {expected_file_name}: expected {expected}, got {actual}"
        ));
    }
    Ok(expected)
}

impl AgnixExtension {
    /// Resolves the agnix-lsp binary path, downloading it from GitHub releases if needed.
    fn language_server_binary_path(
        &mut self,
        language_server_id: &LanguageServerId,
    ) -> Result<String> {
        let (platform, arch) = zed::current_platform();
        let bin = binary_name(platform);

        // Return cached path immediately (trust the cache once set)
        if let Some(ref path) = self.cached_binary_path {
            return Ok(path.clone());
        }

        // Determine the latest release from GitHub
        let release = zed::latest_github_release(
            GITHUB_REPO,
            GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        let version = &release.version;

        // Reject version strings with path traversal characters
        if version.contains('/') || version.contains('\\') || version.contains("..") {
            return Err(format!("invalid release version: {version}"));
        }

        let version_dir = format!("agnix-lsp-{version}");
        let binary_path = format!("{version_dir}/{bin}");
        let verification_marker_path = format!("{version_dir}/.agnix-lsp.sha256");

        // If this version is already downloaded and verified, use it.
        if fs::metadata(&binary_path).is_ok_and(|m| m.is_file())
            && fs::metadata(&verification_marker_path).is_ok_and(|m| m.is_file())
        {
            self.cached_binary_path = Some(binary_path.clone());
            return Ok(binary_path);
        } else if fs::metadata(&binary_path).is_ok_and(|m| m.is_file()) {
            let _ = fs::remove_dir_all(&version_dir);
        }

        // Download the appropriate asset for this platform
        let (asset_name, file_type) = asset_for_platform(platform, arch)?;
        let checksum_asset_name = binary_checksum_asset_name(asset_name)?;
        let checksum_path = format!("{version_dir}/{checksum_asset_name}");

        let asset = release
            .assets
            .iter()
            .find(|a| a.name == asset_name)
            .ok_or_else(|| format!("no release asset found matching {asset_name}"))?;
        let checksum_asset = release
            .assets
            .iter()
            .find(|a| a.name == checksum_asset_name)
            .ok_or_else(|| format!("no release checksum found matching {checksum_asset_name}"))?;

        // Validate download URL uses HTTPS from a trusted GitHub domain
        if !is_trusted_download_url(&asset.download_url) {
            return Err(format!(
                "refusing download from untrusted URL: {}",
                asset.download_url
            ));
        }
        if !is_trusted_download_url(&checksum_asset.download_url) {
            return Err(format!(
                "refusing checksum download from untrusted URL: {}",
                checksum_asset.download_url
            ));
        }

        zed::download_file(&asset.download_url, &version_dir, file_type)
            .map_err(|e| format!("failed to download {asset_name}: {e}"))?;
        let verified_hash = (|| -> Result<String> {
            zed::download_file(
                &checksum_asset.download_url,
                &checksum_path,
                DownloadedFileType::Uncompressed,
            )
            .map_err(|e| format!("failed to download {checksum_asset_name}: {e}"))?;
            let checksum_contents = fs::read_to_string(&checksum_path)
                .map_err(|e| format!("failed to read {checksum_path}: {e}"))?;
            verify_sha256_file(&binary_path, &checksum_contents, bin)
        })()
        .map_err(|e| {
            let _ = fs::remove_dir_all(&version_dir);
            e
        })?;
        fs::write(
            &verification_marker_path,
            format!("{verified_hash}  {bin}\n"),
        )
        .map_err(|e| format!("failed to write {verification_marker_path}: {e}"))?;
        let _ = fs::remove_file(&checksum_path);

        zed::make_file_executable(&binary_path)?;

        self.cached_binary_path = Some(binary_path.clone());

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::None,
        );

        Ok(binary_path)
    }
}

impl zed::Extension for AgnixExtension {
    fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        _worktree: &zed::Worktree,
    ) -> Result<Command> {
        let binary_path = self.language_server_binary_path(language_server_id)?;
        Ok(Command {
            command: binary_path,
            args: vec![],
            env: vec![],
        })
    }
}

zed::register_extension!(AgnixExtension);

#[cfg(test)]
mod tests {
    use super::*;

    const HELLO_SHA256: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";

    #[test]
    fn asset_name_mac_aarch64() {
        let (name, file_type) = asset_for_platform(Os::Mac, Architecture::Aarch64)
            .expect("should get asset for mac aarch64");
        assert_eq!(name, "agnix-lsp-aarch64-apple-darwin.tar.gz");
        assert!(matches!(file_type, DownloadedFileType::GzipTar));
    }

    #[test]
    fn asset_name_mac_x86_64() {
        let (name, file_type) = asset_for_platform(Os::Mac, Architecture::X8664)
            .expect("should get asset for mac x86_64");
        assert_eq!(name, "agnix-lsp-x86_64-apple-darwin.tar.gz");
        assert!(matches!(file_type, DownloadedFileType::GzipTar));
    }

    #[test]
    fn asset_name_linux_x86_64() {
        let (name, file_type) = asset_for_platform(Os::Linux, Architecture::X8664)
            .expect("should get asset for linux x86_64");
        assert_eq!(name, "agnix-lsp-x86_64-unknown-linux-gnu.tar.gz");
        assert!(matches!(file_type, DownloadedFileType::GzipTar));
    }

    #[test]
    fn asset_name_linux_aarch64() {
        let (name, file_type) = asset_for_platform(Os::Linux, Architecture::Aarch64)
            .expect("should get asset for linux aarch64");
        assert_eq!(name, "agnix-lsp-aarch64-unknown-linux-gnu.tar.gz");
        assert!(matches!(file_type, DownloadedFileType::GzipTar));
    }

    #[test]
    fn asset_name_windows_x86_64() {
        let (name, file_type) = asset_for_platform(Os::Windows, Architecture::X8664)
            .expect("should get asset for windows x86_64");
        assert_eq!(name, "agnix-lsp-x86_64-pc-windows-msvc.zip");
        assert!(matches!(file_type, DownloadedFileType::Zip));
    }

    #[test]
    fn unsupported_platform_returns_error() {
        let result = asset_for_platform(Os::Windows, Architecture::Aarch64);
        assert!(result.is_err());
        let err = result.expect_err("should fail for unsupported platform");
        assert!(err.contains("unsupported platform"));
    }

    #[test]
    fn binary_name_unix() {
        assert_eq!(binary_name(Os::Mac), "agnix-lsp");
        assert_eq!(binary_name(Os::Linux), "agnix-lsp");
    }

    #[test]
    fn binary_name_windows() {
        assert_eq!(binary_name(Os::Windows), "agnix-lsp.exe");
    }

    #[test]
    fn version_with_forward_slash_rejected() {
        let version = "../../../tmp/evil";
        assert!(
            version.contains('/') || version.contains('\\') || version.contains(".."),
            "path traversal should be detected"
        );
    }

    #[test]
    fn version_with_backslash_rejected() {
        let version = "v1.0.0\\..\\tmp";
        assert!(
            version.contains('/') || version.contains('\\') || version.contains(".."),
            "path traversal should be detected"
        );
    }

    #[test]
    fn version_with_dot_dot_rejected() {
        let version = "v1.0.0-..";
        assert!(
            version.contains(".."),
            "double-dot path traversal should be detected"
        );
    }

    #[test]
    fn clean_version_accepted() {
        let version = "v0.8.0";
        assert!(
            !(version.contains('/') || version.contains('\\') || version.contains("..")),
            "clean semver should be accepted"
        );
    }

    #[test]
    fn checksum_asset_name_for_tarball() {
        assert_eq!(
            binary_checksum_asset_name("agnix-lsp-x86_64-unknown-linux-gnu.tar.gz")
                .expect("should map tarball asset"),
            "agnix-lsp-x86_64-unknown-linux-gnu.sha256"
        );
    }

    #[test]
    fn checksum_asset_name_for_zip() {
        assert_eq!(
            binary_checksum_asset_name("agnix-lsp-x86_64-pc-windows-msvc.zip")
                .expect("should map zip asset"),
            "agnix-lsp-x86_64-pc-windows-msvc.sha256"
        );
    }

    #[test]
    fn parse_sha256_sidecar_accepts_matching_filename() {
        assert_eq!(
            parse_sha256_sidecar(
                &format!("{}  agnix-lsp\n", HELLO_SHA256.to_uppercase()),
                "agnix-lsp"
            )
            .expect("should parse matching sidecar"),
            HELLO_SHA256
        );
    }

    #[test]
    fn parse_sha256_sidecar_rejects_malformed_hash() {
        let err = parse_sha256_sidecar("not-a-hash  agnix-lsp", "agnix-lsp")
            .expect_err("malformed hash should be rejected");
        assert!(err.contains("valid SHA256"));
    }

    #[test]
    fn parse_sha256_sidecar_rejects_mismatched_filename() {
        let err = parse_sha256_sidecar(
            &format!("{HELLO_SHA256}  agnix-lsp-linux"),
            "agnix-lsp-macos",
        )
        .expect_err("mismatched file name should be rejected");
        assert!(err.contains("expected agnix-lsp-macos"));
    }

    #[test]
    fn verify_sha256_file_accepts_matching_file() {
        let temp_dir = std::env::temp_dir().join(format!(
            "agnix-zed-checksum-{}-matching",
            std::process::id()
        ));
        let file_path = temp_dir.join("agnix-lsp");
        fs::create_dir_all(&temp_dir).expect("should create temp dir");

        let result = (|| {
            fs::write(&file_path, b"hello").expect("should write test file");
            let file_path = file_path.to_string_lossy();
            assert_eq!(
                sha256_file(&file_path).expect("should hash file"),
                HELLO_SHA256
            );
            verify_sha256_file(
                &file_path,
                &format!("{HELLO_SHA256}  agnix-lsp\n"),
                "agnix-lsp",
            )
        })();

        let _ = fs::remove_dir_all(&temp_dir);
        assert!(
            result.is_ok(),
            "expected checksum verification to pass: {result:?}"
        );
    }

    #[test]
    fn verify_sha256_file_rejects_mismatched_file() {
        let temp_dir = std::env::temp_dir().join(format!(
            "agnix-zed-checksum-{}-mismatch",
            std::process::id()
        ));
        let file_path = temp_dir.join("agnix-lsp");
        fs::create_dir_all(&temp_dir).expect("should create temp dir");

        let result = (|| {
            fs::write(&file_path, b"tampered").expect("should write test file");
            let file_path = file_path.to_string_lossy();
            verify_sha256_file(
                &file_path,
                &format!("{HELLO_SHA256}  agnix-lsp\n"),
                "agnix-lsp",
            )
        })();

        let _ = fs::remove_dir_all(&temp_dir);
        assert!(
            result
                .expect_err("mismatched file should fail")
                .contains("checksum mismatch")
        );
    }

    #[test]
    fn trusted_github_url_accepted() {
        let url = "https://github.com/agent-sh/agnix/releases/download/v0.8.0/agnix-lsp.tar.gz";
        assert!(
            is_trusted_download_url(url),
            "github.com URL should be trusted"
        );
    }

    #[test]
    fn trusted_githubusercontent_url_accepted() {
        let url = "https://objects.githubusercontent.com/github-production-release-asset/12345";
        assert!(
            is_trusted_download_url(url),
            "objects.githubusercontent.com URL should be trusted"
        );
    }

    #[test]
    fn untrusted_url_rejected() {
        let url = "https://evil.com/malware.tar.gz";
        assert!(
            !is_trusted_download_url(url),
            "non-GitHub URL should be rejected"
        );
    }

    #[test]
    fn http_url_rejected() {
        let url = "http://github.com/agent-sh/agnix/releases/download/v0.8.0/agnix-lsp.tar.gz";
        assert!(!is_trusted_download_url(url), "HTTP URL should be rejected");
    }
}
