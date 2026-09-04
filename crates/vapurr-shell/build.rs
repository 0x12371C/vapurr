//! Stamp vapurr.ico onto the PE so Explorer / the taskbar show the cat.

use std::path::PathBuf;
use std::process::Command;

fn windres() -> PathBuf {
    if let Ok(dir) = std::env::var("WINLIBS_BIN") {
        let w = PathBuf::from(dir).join("windres.exe");
        if w.exists() {
            return w;
        }
    }
    if let Ok(home) = std::env::var("USERPROFILE") {
        let w = PathBuf::from(home)
            .join("winlibs")
            .join("mingw64")
            .join("bin")
            .join("windres.exe");
        if w.exists() {
            return w;
        }
    }
    PathBuf::from("windres")
}

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let ico = manifest_dir.join("vapurr.ico");
    let app_manifest = manifest_dir.join("vapurr.manifest");
    println!("cargo:rerun-if-changed={}", ico.display());
    println!("cargo:rerun-if-changed={}", app_manifest.display());
    if !ico.exists() {
        println!("cargo:warning=vapurr.ico missing; exe will ship without an icon");
        return;
    }

    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let rc = out.join("vapurr.rc");
    let obj = out.join("vapurr_icon.o");
    let ico_posix = ico.display().to_string().replace('\\', "/");
    let _ = app_manifest;
    let ver = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.1.0".into());
    let parts: Vec<&str> = ver.split('.').collect();
    let maj = parts.first().copied().unwrap_or("0");
    let min = parts.get(1).copied().unwrap_or("0");
    let pat = parts.get(2).copied().unwrap_or("0");
    let rc_txt = format!(
        "1 ICON \"{ico_posix}\"\n\
         1 VERSIONINFO\n\
         FILEVERSION {maj},{min},{pat},0\n\
         PRODUCTVERSION {maj},{min},{pat},0\n\
         FILEFLAGSMASK 0x3fL\n\
         FILEFLAGS 0\n\
         FILEOS 0x40004L\n\
         FILETYPE 0x1L\n\
         BEGIN\n\
         BLOCK \"StringFileInfo\"\n\
         BEGIN\n\
         BLOCK \"040904b0\"\n\
         BEGIN\n\
         VALUE \"CompanyName\", \"vapurr\"\n\
         VALUE \"FileDescription\", \"vapurr\"\n\
         VALUE \"FileVersion\", \"{ver}\"\n\
         VALUE \"InternalName\", \"vapurr\"\n\
         VALUE \"OriginalFilename\", \"vapurr.exe\"\n\
         VALUE \"ProductName\", \"vapurr\"\n\
         VALUE \"ProductVersion\", \"{ver}\"\n\
         VALUE \"LegalCopyright\", \"Copyright (C) 2026 vapurr\"\n\
         END\n\
         END\n\
         BLOCK \"VarFileInfo\"\n\
         BEGIN\n\
         VALUE \"Translation\", 0x409, 1200\n\
         END\n\
         END\n"
    );
    std::fs::write(&rc, rc_txt).expect("write rc");

    let status = Command::new(windres())
        .args([
            "--input",
            rc.to_str().unwrap(),
            "--output",
            obj.to_str().unwrap(),
            "--output-format=coff",
        ])
        .status()
        .expect("spawn windres");
    if !status.success() {
        panic!("windres failed: {status}");
    }
    println!("cargo:rustc-link-arg-bins={}", obj.display());
}
