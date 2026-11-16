use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=icons");
    println!("cargo:rerun-if-changed=../vendor/mpv/libmpv-2.dll");

    // `tauri dev` / `cargo run` load libmpv-2.dll from next to the exe: copy it into
    // target/<profile>/ (the installer gets it through bundle.resources).
    let dll = PathBuf::from("../vendor/mpv/libmpv-2.dll");
    if dll.exists() {
        let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
        // OUT_DIR = target/<profile>/build/videk-<hash>/out
        if let Some(profile_dir) = out.ancestors().nth(3) {
            let dst = profile_dir.join("libmpv-2.dll");
            let stale = match (dll.metadata(), dst.metadata()) {
                (Ok(a), Ok(b)) => a.len() != b.len() || a.modified().ok() > b.modified().ok(),
                _ => true,
            };
            if stale {
                let _ = std::fs::copy(&dll, &dst);
            }
        }
    } else {
        println!(
            "cargo:warning=vendor/mpv/libmpv-2.dll is missing: run scripts/fetch-libmpv.ps1"
        );
    }
    tauri_build::build()
}
