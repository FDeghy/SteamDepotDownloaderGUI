use std::{env, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=engine-revision");
    println!("cargo:rerun-if-changed=build-engine.mjs");
    println!("cargo:rerun-if-changed=verified-depot-keys.patch");
    let status = Command::new("node")
        .args(["build-engine.mjs", &env::var("TARGET").unwrap()])
        .status()
        .expect("could not start the DepotDownloaderMod source build");
    assert!(
        status.success(),
        "DepotDownloaderMod source build failed: {status}"
    );
    tauri_build::build();
}
