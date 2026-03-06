// ─── Onyx App — Platform-Aware Build Script ────────────────────────
//
// This build script runs before onyx-app is compiled.  It cannot
// influence dependency build scripts (e.g. audiopus_sys/cmake),
// but it performs these duties:
//
// 1. **Diagnostics** — prints clear cargo:warning messages when
//    the build environment is misconfigured for the current target.
//
// 2. **Android NDK discovery** — if ANDROID_NDK_HOME is missing
//    when cross-compiling for Android, attempts to locate the NDK
//    installed by `cargo makepad android install-toolchain` and
//    re-exports it so downstream proc-macros and codegen see it.
//
// 3. **Windows cmake guard** — verifies CMAKE_GENERATOR is set
//    (via .cargo/config.toml fallback) so audiopus_sys can
//    configure with a known VS generator.
//
// Note: environment variables set here (via `cargo:rustc-env=`)
// only affect the *current* crate's compilation, not dependencies.
// The real fix for the cmake generator conflict lives in two places:
//   • .cargo/config.toml  — `force = false` fallback for Windows
//   • cargo-makepad patch — exports ANDROID_NDK_HOME + CMAKE_GENERATOR=Ninja
// ────────────────────────────────────────────────────────────────────

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

    match target_os.as_str() {
        "android" => configure_android(&target_arch),
        "windows" => configure_windows(),
        _ => {} // macOS, Linux — no special config needed
    }
}

/// Android: verify NDK and cmake generator are correctly set.
fn configure_android(target_arch: &str) {
    println!("cargo:rerun-if-env-changed=ANDROID_NDK_HOME");
    println!("cargo:rerun-if-env-changed=CMAKE_GENERATOR");

    // ── Check CMAKE_GENERATOR ──
    match std::env::var("CMAKE_GENERATOR") {
        Ok(gen) if gen.contains("Visual Studio") => {
            println!(
                "cargo:warning=CMAKE_GENERATOR is set to '{}' but target is Android. \
                 This will fail. Set CMAKE_GENERATOR=Ninja before running \
                 `cargo makepad android run`.",
                gen
            );
        }
        Ok(gen) => {
            println!(
                "cargo:warning=[onyx-app build.rs] Android build — CMAKE_GENERATOR={}",
                gen
            );
        }
        Err(_) => {
            println!(
                "cargo:warning=[onyx-app build.rs] CMAKE_GENERATOR not set; \
                 cmake will auto-detect (should be fine for Android NDK builds)."
            );
        }
    }

    // ── Check ANDROID_NDK_HOME ──
    if std::env::var("ANDROID_NDK_HOME").is_ok() {
        return; // Already set — nothing to do.
    }

    // Attempt to locate the makepad-installed NDK.
    let home = if cfg!(windows) {
        std::env::var("USERPROFILE").unwrap_or_default()
    } else {
        std::env::var("HOME").unwrap_or_default()
    };

    if home.is_empty() {
        println!(
            "cargo:warning=[onyx-app build.rs] ANDROID_NDK_HOME is not set and \
             HOME is unknown. cmake-based dependencies may fail to build. \
             Run `cargo makepad android install-toolchain` first."
        );
        return;
    }

    let ndk_root = std::path::PathBuf::from(&home)
        .join(".makepad")
        .join("android")
        .join("ndk");

    if !ndk_root.is_dir() {
        println!(
            "cargo:warning=[onyx-app build.rs] ANDROID_NDK_HOME is not set and \
             {:?} does not exist. Run `cargo makepad android install-toolchain`.",
            ndk_root
        );
        return;
    }

    // Find the newest NDK version directory.
    let mut best: Option<std::path::PathBuf> = None;
    if let Ok(entries) = std::fs::read_dir(&ndk_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                best = Some(path);
            }
        }
    }

    if let Some(ndk_path) = best {
        println!(
            "cargo:warning=[onyx-app build.rs] Auto-discovered NDK at {:?} \
             (target: {}-android). Note: this only affects onyx-app's own \
             compilation env, not dependency build scripts.",
            ndk_path, target_arch
        );
        // Export for this crate's own use (informational).
        println!("cargo:rustc-env=ANDROID_NDK_HOME={}", ndk_path.display());
    } else {
        println!(
            "cargo:warning=[onyx-app build.rs] NDK directory {:?} exists but \
             contains no version subdirectories.",
            ndk_root
        );
    }
}

/// Windows: verify CMAKE_GENERATOR is set for VS compatibility.
fn configure_windows() {
    println!("cargo:rerun-if-env-changed=CMAKE_GENERATOR");

    match std::env::var("CMAKE_GENERATOR") {
        Ok(gen) => {
            println!(
                "cargo:warning=[onyx-app build.rs] Windows build — CMAKE_GENERATOR={}",
                gen
            );
        }
        Err(_) => {
            println!(
                "cargo:warning=[onyx-app build.rs] CMAKE_GENERATOR is not set. \
                 cmake will auto-detect the Visual Studio generator. If the build \
                 fails with 'generator not found', set CMAKE_GENERATOR='Visual Studio 17 2022' \
                 in your shell or .cargo/config.toml."
            );
        }
    }
}
