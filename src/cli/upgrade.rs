use std::io::Read;
use std::path::PathBuf;

use crate::engine::upgrade::{check_latest_version, current_target};

pub fn run_upgrade(dry_run: bool) -> i32 {
    let spinner = cliclack::spinner();
    spinner.start("Checking for updates...");

    let info = match check_latest_version() {
        Some(info) => info,
        None => {
            spinner.stop("Check failed");
            cliclack::log::error(
                "Could not check for updates. Check your network connection and try again.",
            )
            .ok();
            return 1;
        }
    };

    if !info.update_available {
        spinner.stop(format!("v{} is the latest version", info.current));
        return 0;
    }

    let download_url = match &info.download_url {
        Some(url) => url.clone(),
        None => {
            spinner.stop("Update available");
            cliclack::log::error("No download URL available for this platform.").ok();
            return 1;
        }
    };

    spinner.stop(format!("Update available: v{} \u{2192} v{}", info.current, info.latest));

    if dry_run {
        let exe_path = std::env::current_exe()
            .ok()
            .and_then(|p| p.canonicalize().ok())
            .unwrap_or_else(|| PathBuf::from("sk"));
        cliclack::log::info(format!("Download URL: {download_url}")).ok();
        cliclack::log::info(format!("Binary path:  {}", exe_path.display())).ok();
        cliclack::log::info("Dry run — no changes made.").ok();
        return 0;
    }

    let target = match current_target() {
        Some(t) => t,
        None => {
            cliclack::log::error("Unsupported platform. Cannot determine download target.").ok();
            return 1;
        }
    };

    let exe_path = match std::env::current_exe().and_then(|p| p.canonicalize()) {
        Ok(p) => p,
        Err(e) => {
            cliclack::log::error(format!("Cannot determine binary path: {e}")).ok();
            return 1;
        }
    };

    let dl_spinner = cliclack::spinner();
    dl_spinner.start("Downloading update...");

    let response = match ureq::get(&download_url).set("User-Agent", "skald-cli").call() {
        Ok(r) => r,
        Err(e) => {
            dl_spinner.stop("Download failed");
            cliclack::log::error(format!("Failed to download update: {e}")).ok();
            return 1;
        }
    };

    let mut body = Vec::new();
    if let Err(e) = response.into_reader().read_to_end(&mut body) {
        dl_spinner.stop("Download failed");
        cliclack::log::error(format!("Failed to read response: {e}")).ok();
        return 1;
    }

    dl_spinner.stop("Download complete");

    let extract_spinner = cliclack::spinner();
    extract_spinner.start("Installing update...");

    let tmpdir = match tempfile::tempdir() {
        Ok(d) => d,
        Err(e) => {
            extract_spinner.stop("Extract failed");
            cliclack::log::error(format!("Failed to create temp directory: {e}")).ok();
            return 1;
        }
    };

    let decoder = xz2::read::XzDecoder::new(&body[..]);
    let mut archive = tar::Archive::new(decoder);
    if let Err(e) = archive.unpack(tmpdir.path()) {
        extract_spinner.stop("Extract failed");
        cliclack::log::error(format!("Failed to extract archive: {e}")).ok();
        return 1;
    }

    let binary_name = if cfg!(target_os = "windows") { "sk.exe" } else { "sk" };
    let extracted_binary = find_binary(tmpdir.path(), binary_name);

    let extracted_binary = match extracted_binary {
        Some(p) => p,
        None => {
            extract_spinner.stop("Extract failed");
            cliclack::log::error(format!(
                "Could not find '{binary_name}' in the downloaded archive. \
                 Expected target: {target}"
            ))
            .ok();
            return 1;
        }
    };

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        if let Err(e) = std::fs::set_permissions(&extracted_binary, perms) {
            tracing::warn!("Failed to set executable permissions: {e}");
        }
    }

    if let Err(rename_err) = std::fs::rename(&extracted_binary, &exe_path) {
        tracing::debug!("Rename failed (expected if cross-device): {rename_err}");
        if let Err(copy_err) = std::fs::copy(&extracted_binary, &exe_path) {
            extract_spinner.stop("Install failed");
            cliclack::log::error(format!(
                "Cannot update {}: {copy_err}. Try running with elevated permissions.",
                exe_path.display()
            ))
            .ok();
            return 1;
        }
    }

    extract_spinner.stop(format!("Updated: v{} \u{2192} v{}", info.current, info.latest));

    0
}

fn find_binary(dir: &std::path::Path, name: &str) -> Option<PathBuf> {
    for entry in walkdir(dir) {
        if let Some(file_name) = entry.file_name()
            && file_name == name
        {
            return Some(entry);
        }
    }
    None
}

fn walkdir(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut results = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                results.extend(walkdir(&path));
            } else {
                results.push(path);
            }
        }
    }
    results
}
