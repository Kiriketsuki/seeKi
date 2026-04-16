use std::{fs, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=VERSION");
    println!("cargo:rerun-if-changed=.git/HEAD");

    let version = fs::read_to_string("VERSION")
        .unwrap_or_else(|_| "unknown".to_string())
        .trim()
        .to_string();
    println!("cargo:rustc-env=SEEKI_VERSION={version}");

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
}
