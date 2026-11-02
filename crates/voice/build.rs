//! macOS: link clang's compiler runtime (libclang_rt.osx.a). whisper.cpp's
//! Metal backend uses `@available`, which compiles to calls to
//! `__isPlatformVersionAtLeast` whenever the deployment target is older than
//! the version it checks. The release builds for macOS 12.0
//! (scripts/release-mac.sh), so the calls stay, and rustc's link line does
//! not include clang's runtime, so the link failed. Dev builds target the
//! host's macOS and the checks compile away, which is why only the release
//! hit it.

use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    let out = Command::new("xcrun")
        .args(["clang", "--print-runtime-dir"])
        .output()
        .or_else(|_| Command::new("clang").arg("--print-runtime-dir").output());
    match out {
        Ok(o) if o.status.success() => {
            let dir = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if std::path::Path::new(&dir)
                .join("libclang_rt.osx.a")
                .exists()
            {
                println!("cargo:rustc-link-search=native={dir}");
                println!("cargo:rustc-link-lib=static=clang_rt.osx");
            } else {
                println!("cargo:warning=no libclang_rt.osx.a in {dir}");
            }
        }
        _ => println!("cargo:warning=clang --print-runtime-dir failed; voice may not link"),
    }
}
