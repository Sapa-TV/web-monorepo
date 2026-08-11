use std::env::var;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let git_sha = var("GIT_SHA")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            let output = Command::new("git")
                .args(["rev-parse", "--short", "HEAD"])
                .output()
                .ok()?;
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        });

    if let Some(sha) = git_sha {
        println!("cargo:rustc-env=GIT_SHA={sha}");
    }
}
