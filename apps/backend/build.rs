use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    if let Ok(output) = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        && output.status.success()
    {
        let sha = String::from_utf8_lossy(&output.stdout);
        println!("cargo:rustc-env=GIT_SHA={}", sha.trim());
    }
}
