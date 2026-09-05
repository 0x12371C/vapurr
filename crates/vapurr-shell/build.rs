//! Stamp vapurr.ico onto the PE so Explorer / the taskbar show the cat.
//! Embed WebView2Loader.dll and replace the gnu import lib with a LoadLibrary shim
//! so lone setup.exe can start (hard PE import otherwise fails before main).

use std::path::{Path, PathBuf};
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

fn tool(name: &str) -> PathBuf {
    if let Ok(dir) = std::env::var("WINLIBS_BIN") {
        let p = PathBuf::from(dir).join(name);
        if p.exists() {
            return p;
        }
    }
    if let Ok(home) = std::env::var("USERPROFILE") {
        let p = PathBuf::from(home)
            .join("winlibs")
            .join("mingw64")
            .join("bin")
            .join(name);
        if p.exists() {
            return p;
        }
    }
    PathBuf::from(name)
}

fn target_arch_dir() -> &'static str {
    match std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default().as_str() {
        "x86_64" => "x64",
        "x86" => "x86",
        "aarch64" => "arm64",
        _ => "x64",
    }
}

fn find_webview2_x64_dir() -> Option<PathBuf> {
    let out = PathBuf::from(std::env::var("OUT_DIR").ok()?);
    let arch = target_arch_dir();
    let release = out.ancestors().nth(3)?;
    let build = release.join("build");
    if let Ok(walk) = std::fs::read_dir(&build) {
        for ent in walk.flatten() {
            let d = ent.path().join("out").join(arch);
            if d.join("WebView2Loader.dll").is_file() {
                return Some(d);
            }
        }
    }
    None
}

fn find_webview2_loader() -> Option<PathBuf> {
    find_webview2_x64_dir().map(|d| d.join("WebView2Loader.dll"))
}

const WEBVIEW2_SHIM_C: &str = r#"
#include <windows.h>

typedef HRESULT (WINAPI *fn_CompareBrowserVersions)(PCWSTR, PCWSTR, int*);
typedef HRESULT (WINAPI *fn_CreateCoreWebView2Environment)(void*);
typedef HRESULT (WINAPI *fn_CreateCoreWebView2EnvironmentWithOptions)(PCWSTR, PCWSTR, void*, void*);
typedef HRESULT (WINAPI *fn_GetAvailableCoreWebView2BrowserVersionString)(PCWSTR, PWSTR*);
typedef HRESULT (WINAPI *fn_GetAvailableCoreWebView2BrowserVersionStringWithOptions)(PCWSTR, void*, PWSTR*);

static HMODULE load_webview2_loader(void) {
    static HMODULE h;
    wchar_t path[MAX_PATH];
    DWORD n, i;
    if (h) return h;
    n = GetModuleFileNameW(NULL, path, MAX_PATH);
    if (n > 0 && n < MAX_PATH) {
        for (i = n; i > 0; i--) {
            if (path[i - 1] == L'\\' || path[i - 1] == L'/') {
                const wchar_t *name = L"WebView2Loader.dll";
                DWORD j = 0;
                while (name[j] && (i + j) < MAX_PATH) {
                    path[i + j] = name[j];
                    j++;
                }
                if ((i + j) < MAX_PATH) {
                    path[i + j] = 0;
                    h = LoadLibraryW(path);
                }
                break;
            }
        }
    }
    if (!h) h = LoadLibraryW(L"WebView2Loader.dll");
    return h;
}

static void *loader_proc(const char *name) {
    HMODULE mod = load_webview2_loader();
    if (!mod) return NULL;
    return (void*)GetProcAddress(mod, name);
}

HRESULT WINAPI CompareBrowserVersions(PCWSTR v1, PCWSTR v2, int *result) {
    fn_CompareBrowserVersions fn = (fn_CompareBrowserVersions)loader_proc("CompareBrowserVersions");
    if (!fn) return E_FAIL;
    return fn(v1, v2, result);
}

HRESULT WINAPI CreateCoreWebView2Environment(void *handler) {
    fn_CreateCoreWebView2Environment fn =
        (fn_CreateCoreWebView2Environment)loader_proc("CreateCoreWebView2Environment");
    if (!fn) return E_FAIL;
    return fn(handler);
}

HRESULT WINAPI CreateCoreWebView2EnvironmentWithOptions(
    PCWSTR browser, PCWSTR user_data, void *opts, void *handler) {
    fn_CreateCoreWebView2EnvironmentWithOptions fn =
        (fn_CreateCoreWebView2EnvironmentWithOptions)loader_proc(
            "CreateCoreWebView2EnvironmentWithOptions");
    if (!fn) return E_FAIL;
    return fn(browser, user_data, opts, handler);
}

HRESULT WINAPI GetAvailableCoreWebView2BrowserVersionString(PCWSTR browser, PWSTR *version) {
    fn_GetAvailableCoreWebView2BrowserVersionString fn =
        (fn_GetAvailableCoreWebView2BrowserVersionString)loader_proc(
            "GetAvailableCoreWebView2BrowserVersionString");
    if (!fn) return E_FAIL;
    return fn(browser, version);
}

HRESULT WINAPI GetAvailableCoreWebView2BrowserVersionStringWithOptions(
    PCWSTR browser, void *opts, PWSTR *version) {
    fn_GetAvailableCoreWebView2BrowserVersionStringWithOptions fn =
        (fn_GetAvailableCoreWebView2BrowserVersionStringWithOptions)loader_proc(
            "GetAvailableCoreWebView2BrowserVersionStringWithOptions");
    if (!fn) return E_FAIL;
    return fn(browser, opts, version);
}
"#;

fn embed_loader(out: &Path) {
    let Some(src) = find_webview2_loader() else {
        println!("cargo:warning=WebView2Loader.dll not found under target build; setup embed skipped");
        std::fs::write(
            out.join("webview2_loader_bytes.rs"),
            "pub static BYTES: &[u8] = &[];\n",
        )
        .expect("write empty loader bytes");
        return;
    };
    println!("cargo:rerun-if-changed={}", src.display());
    let dest = out.join("WebView2Loader.dll");
    std::fs::copy(&src, &dest).expect("copy WebView2Loader.dll to OUT_DIR");
    let dest_posix = dest.display().to_string().replace('\\', "/");
    std::fs::write(
        out.join("webview2_loader_bytes.rs"),
        format!("pub static BYTES: &[u8] = include_bytes!(\"{dest_posix}\");\n"),
    )
    .expect("write webview2_loader_bytes.rs");

    let shim_c = out.join("webview2_shim.c");
    std::fs::write(&shim_c, WEBVIEW2_SHIM_C).expect("write shim c");
    let shim_o = out.join("webview2_shim.o");
    let st = Command::new(tool("gcc.exe"))
        .args([
            "-c",
            "-O2",
            "-o",
            shim_o.to_str().unwrap(),
            shim_c.to_str().unwrap(),
        ])
        .status()
        .expect("spawn gcc for webview2 shim");
    if !st.success() {
        panic!("gcc webview2 shim failed: {st}");
    }

    // Replace webview2-com-sys import lib so -lWebView2Loader.dll resolves to our
    // LoadLibrary shim instead of a hard PE import (lone setup.exe must start).
    let Some(x64) = find_webview2_x64_dir() else {
        println!("cargo:warning=webview2 x64 dir missing; linking shim.o directly");
        println!("cargo:rustc-link-arg={}", shim_o.display());
        println!("cargo:rustc-link-arg=-Wl,--allow-multiple-definition");
        return;
    };
    let import_lib = x64.join("WebView2Loader.dll.lib");
    let backup = x64.join("WebView2Loader.dll.lib.orig");
    if import_lib.is_file() && !backup.is_file() {
        let _ = std::fs::copy(&import_lib, &backup);
    }
    let ar_lib = out.join("WebView2Loader.dll.lib");
    let _ = std::fs::remove_file(&ar_lib);
    let st = Command::new(tool("ar.exe"))
        .args([
            "rcs",
            ar_lib.to_str().unwrap(),
            shim_o.to_str().unwrap(),
        ])
        .status()
        .expect("spawn ar for webview2 shim lib");
    if !st.success() {
        panic!("ar webview2 shim lib failed: {st}");
    }
    std::fs::copy(&ar_lib, &import_lib).expect("overwrite WebView2Loader.dll.lib with shim");
    println!(
        "cargo:warning=webview2: replaced import lib with LoadLibrary shim at {}",
        import_lib.display()
    );
}

fn stamp_icon(out: &Path) {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let ico = manifest_dir.join("vapurr.ico");
    let app_manifest = manifest_dir.join("vapurr.manifest");
    println!("cargo:rerun-if-changed={}", ico.display());
    println!("cargo:rerun-if-changed={}", app_manifest.display());
    if !ico.exists() {
        println!("cargo:warning=vapurr.ico missing; exe will ship without an icon");
        return;
    }

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

fn main() {
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    #[cfg(windows)]
    {
        embed_loader(&out);
    }
    #[cfg(not(windows))]
    {
        let _ = &out;
    }
    stamp_icon(&out);
}
