extern crate bindgen;

use std::env;
use std::fs;
use std::path::PathBuf;

#[cfg(windows)]
mod build_win;

#[cfg(target_os = "macos")]
mod build_mac;

fn main() {
    // Tell cargo to invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed=wrapper.h");

    #[cfg(not(target_os = "macos"))] {
        println!("cargo:rustc-link-lib=dylib=obs");
        println!("cargo:rustc-link-lib=dylib=obs-frontend-api");
    }

    #[cfg(target_os = "macos")]
    build_mac::find_mac_obs_lib();

    #[cfg(windows)]
    build_win::find_windows_obs_lib();

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let input = "wrapper.h";

    let bindings = bindgen::Builder::default()
        .header(input)
        .clang_args([
            "-I./obs/libobs/",
            "-I./obs/UI/obs-frontend-api/",
        ])
        .blocklist_function("_.*")
        // following 3 use _udiv128 which doesn't link
        .blocklist_function("util_mul_div64")
        .blocklist_function("audio_frames_to_ns")
        .blocklist_function("ns_to_audio_frames")
        .derive_default(true)
        .wrap_static_fns(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate().unwrap();

    bindings.write_to_file(out_path.join("bindings.rs")).expect("Couldn't write bindings!");

    let obj_path = out_path.join("extern.o");

    let clang_output = std::process::Command::new("clang")
        .arg("-flto=thin")
        .arg("-O")
        .arg("-c")
        .arg("-o")
        .arg(&obj_path)
        .arg(std::env::temp_dir().join("bindgen").join("extern.c"))
        .arg("-include")
        .arg(input)
        .arg("-I./")
        .arg("-I./obs/libobs/")
        .arg("-I./obs/UI/obs-frontend-api/")
        .output().unwrap();
        
    if !clang_output.status.success() {
        panic!("Could not compile object file:\n{}", String::from_utf8_lossy(&clang_output.stderr));
    }

    // if you have clang in path, you probably have llvm-lib
    let mut lib = std::process::Command::new("llvm-lib");

    #[cfg(not(target_os = "windows"))]
    let lib_output = Command::new("ar")
        .arg("rcs")
        .arg(out_dir_path.join("libextern.a"))
        .arg(obj_path)
        .output().unwrap();
    #[cfg(target_os = "windows")]
    let lib_output = lib.arg(obj_path).output().unwrap();

    if !lib_output.status.success() {
        panic!("Could not emit library file:\n{}", String::from_utf8_lossy(&lib_output.stderr));
    }

    println!("cargo:rustc-link-search=native={}", out_path.display());
    println!("cargo:rustc-link-lib=static=extern");
}
