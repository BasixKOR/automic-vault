use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }

    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let object = out.join("xpc_shim.o");
    let library = out.join("libav_xpc_shim.a");

    assert!(
        Command::new("cc")
            .args(["-fblocks", "-c", "src/cli/xpc_shim.c", "-o"])
            .arg(&object)
            .status()
            .unwrap()
            .success(),
        "failed to compile XPC shim"
    );
    assert!(
        Command::new("ar")
            .args(["crs"])
            .arg(&library)
            .arg(&object)
            .status()
            .unwrap()
            .success(),
        "failed to archive XPC shim"
    );

    println!("cargo:rerun-if-changed=src/cli/xpc_shim.c");
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=av_xpc_shim");
    println!("cargo:rustc-link-arg-bin=av-brew-stub=-lav_xpc_shim");
}
