use std::process::Command;

fn main() {
    // Read VERSION file
    let version = std::fs::read_to_string("VERSION")
        .unwrap_or_else(|_| "0.0.0.0".to_string())
        .trim()
        .to_string();
    println!("cargo:rustc-env=SEEKI_VERSION={version}");

    // Git commit SHA (short)
    let commit = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=SEEKI_COMMIT={commit}");

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

    // Re-run if VERSION or git HEAD changes
    println!("cargo:rerun-if-changed=VERSION");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads");
}
