<<<<<<< HEAD
use std::{fs, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=VERSION");
    println!("cargo:rerun-if-changed=.git/HEAD");

    let version = fs::read_to_string("VERSION")
        .unwrap_or_else(|_| "unknown".to_string())
=======
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=VERSION");
    println!("cargo:rerun-if-changed=.git/HEAD");

    // Read VERSION file
    let version = std::fs::read_to_string("VERSION")
        .unwrap_or_else(|_| "0.0.0.0".to_string())
>>>>>>> 1ea1972 (feat: Add update patcher (auto-check GitHub releases + manual WIP upload) (#71))
        .trim()
        .to_string();
    println!("cargo:rustc-env=SEEKI_VERSION={version}");

<<<<<<< HEAD
    let commit = Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=SEEKI_GIT_COMMIT={commit}");

    let built_at = chrono::Utc::now().to_rfc3339();
    println!("cargo:rustc-env=SEEKI_BUILT_AT={built_at}");
=======
    // Git commit SHA (short)
    let commit = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=SEEKI_GIT_COMMIT={commit}");

    // Build timestamp (ISO 8601 UTC)
    let built_at = Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=SEEKI_BUILT_AT={built_at}");

    println!("cargo:rerun-if-changed=.git/refs/heads");
>>>>>>> 1ea1972 (feat: Add update patcher (auto-check GitHub releases + manual WIP upload) (#71))
}
