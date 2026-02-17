use std::env;
use std::path::PathBuf;

fn main() {
    // Tell cargo to rerun this build script if any of these change
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/");

    // Link against Windows libraries
    println!("cargo:rustc-link-lib=ntdll");
    println!("cargo:rustc-link-lib=kernel32");
    println!("cargo:rustc-link-lib=user32");
    println!("cargo:rustc-link-lib=advapi32");
    println!("cargo:rustc-link-lib=shlwapi");

    // Set the subsystem to Windows (GUI) to avoid console window
    println!("cargo:rustc-link-arg=/SUBSYSTEM:WINDOWS");

    // Export DllMain
    println!("cargo:rustc-link-arg=/EXPORT:DllMain");

    // Set the DLL base address (same as original)
    println!("cargo:rustc-link-arg=/BASE:0x180000000");

    // Generate PDB file for debugging
    let out_dir = env::var("OUT_DIR").unwrap();
    let pdb_path = PathBuf::from(&out_dir).join("reflex.pdb");
    println!("cargo:rustc-link-arg=/PDB:{}", pdb_path.display());

    // Set DLL characteristics
    println!("cargo:rustc-link-arg=/DYNAMICBASE"); // ASLR
    println!("cargo:rustc-link-arg=/NXCOMPAT");    // DEP

    // Optimization flags for release builds
    if env::var("PROFILE").unwrap() == "release" {
        println!("cargo:rustc-link-arg=/OPT:REF");
        println!("cargo:rustc-link-arg=/OPT:ICF");
    }
}
