//! Sync this binary with a channel build and swap it in without the installer.
//!
//! Channel: `%LOCALAPPDATA%\vapurr\channel\` (manifest.json + vapurr.exe).
//! Sibling `vapurr.next.exe` is also a candidate (dev stage). Apply copies the
//! new file beside the running exe, launches `--patch-swap`, then exits.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::crash;
use crate::setup;

const BREAKAWAY: u32 = 0x0100_0000;
const DETACH: u32 = 0x0000_0008;
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub build: String,
    #[serde(default)]
    pub rev: String,
    pub sha256: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub target: String,
    #[serde(default = "default_file")]
    pub file: String,
}

fn default_file() -> String {
    "vapurr.exe".into()
}

impl Manifest {
    pub fn short_sha(&self) -> String {
        self.sha256.chars().take(8).collect()
    }
}

pub fn channel_dir() -> PathBuf {
    crash::profile_dir().join("channel")
}

pub fn sha256_path(path: &Path) -> Result<String, String> {
    let mut f = File::open(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = f.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn now_rfc3339() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

pub fn running_exe() -> Result<PathBuf, String> {
    std::env::current_exe().map_err(|e| e.to_string())
}

fn exe_dir() -> PathBuf {
    running_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn next_path() -> PathBuf {
    exe_dir().join("vapurr.next.exe")
}

fn dest_exe() -> PathBuf {
    let installed = setup::install_dir().join("vapurr.exe");
    let me = running_exe().ok();
    if let Some(p) = &me {
        let name = p
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if name == "vapurr.next.exe" {
            return p.parent().unwrap_or(Path::new(".")).join("vapurr.exe");
        }
        if name == "vapurr.exe" {
            return p.clone();
        }
    }
    if installed.is_file() {
        installed
    } else {
        exe_dir().join("vapurr.exe")
    }
}

fn read_manifest(dir: &Path) -> Option<Manifest> {
    let raw = fs::read_to_string(dir.join("manifest.json")).ok()?;
    serde_json::from_str(&raw).ok()
}

fn write_manifest(dir: &Path, m: &Manifest) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let body = serde_json::to_vec_pretty(m).map_err(|e| e.to_string())?;
    let mut f = File::create(dir.join("manifest.json")).map_err(|e| e.to_string())?;
    f.write_all(&body).map_err(|e| e.to_string())?;
    Ok(())
}

fn manifest_for(exe: &Path, rev: &str) -> Result<Manifest, String> {
    let meta = fs::metadata(exe).map_err(|e| e.to_string())?;
    let sha = sha256_path(exe)?;
    Ok(Manifest {
        name: "vapurr".into(),
        version: VERSION.into(),
        build: now_rfc3339(),
        rev: rev.to_string(),
        sha256: sha,
        size: meta.len(),
        target: "x86_64-pc-windows-gnu".into(),
        file: "vapurr.exe".into(),
    })
}

fn copy_if_present(src_dir: &Path, dest_dir: &Path, name: &str) {
    let src = src_dir.join(name);
    if src.is_file() {
        let _ = fs::copy(&src, dest_dir.join(name));
    }
}

/// Stamp this exe into `dir` (default: the user channel).
pub fn publish_to(src_exe: &Path, dest_dir: &Path, rev: &str) -> Result<Manifest, String> {
    fs::create_dir_all(dest_dir).map_err(|e| e.to_string())?;
    let dest_exe = dest_dir.join("vapurr.exe");
    if src_exe.canonicalize().ok() != dest_exe.canonicalize().ok() {
        fs::copy(src_exe, &dest_exe).map_err(|e| format!("copy exe: {e}"))?;
    }
    if let Some(parent) = src_exe.parent() {
        copy_if_present(parent, dest_dir, "WebView2Loader.dll");
        copy_if_present(parent, dest_dir, "VERSION.txt");
    }
    let m = manifest_for(&dest_exe, rev)?;
    write_manifest(dest_dir, &m)?;
    let _ = fs::write(
        dest_dir.join("VERSION.txt"),
        format!("vapurr {}\nrev {}\nsha {}\n", m.version, m.rev, m.sha256),
    );
    crash::log(&format!(
        "published {} {} → {}",
        m.short_sha(),
        m.version,
        dest_dir.display()
    ));
    Ok(m)
}

fn git_rev() -> String {
    let out = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output();
    match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => String::new(),
    }
}

pub fn publish_cli(args: &[String]) -> Result<(), String> {
    let src = running_exe()?;
    let dest = args
        .iter()
        .position(|a| a == "--publish")
        .and_then(|i| args.get(i + 1))
        .filter(|s| !s.starts_with('-'))
        .map(PathBuf::from)
        .unwrap_or_else(channel_dir);
    let m = publish_to(&src, &dest, &git_rev())?;
    println!(
        "published {} {} {}",
        m.version,
        m.short_sha(),
        dest.display()
    );
    Ok(())
}

#[derive(Debug, Clone)]
struct Candidate {
    source: &'static str,
    path: PathBuf,
    manifest: Manifest,
}

fn synthetic_manifest(exe: &Path) -> Option<Manifest> {
    let sha = sha256_path(exe).ok()?;
    let size = fs::metadata(exe).ok()?.len();
    Some(Manifest {
        name: "vapurr".into(),
        version: VERSION.into(),
        build: String::new(),
        rev: String::new(),
        sha256: sha,
        size,
        target: "x86_64-pc-windows-gnu".into(),
        file: exe
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("vapurr.exe")
            .into(),
    })
}

fn candidate_from_dir(dir: &Path, source: &'static str) -> Option<Candidate> {
    let exe = dir.join("vapurr.exe");
    if !exe.is_file() {
        return None;
    }
    let manifest = read_manifest(dir).or_else(|| synthetic_manifest(&exe))?;
    Some(Candidate {
        source,
        path: exe,
        manifest,
    })
}

fn find_repo_dist() -> Option<PathBuf> {
    let mut cur = exe_dir();
    for _ in 0..8 {
        let dist = cur.join("dist").join("vapurr").join("vapurr.exe");
        if dist.is_file() {
            return Some(dist);
        }
        let cargo = cur.join("Cargo.toml");
        if cargo.is_file() {
            let dist = cur.join("dist").join("vapurr").join("vapurr.exe");
            if dist.is_file() {
                return Some(dist);
            }
        }
        if !cur.pop() {
            break;
        }
    }
    None
}

fn candidates() -> Vec<Candidate> {
    let mut out = Vec::new();
    let next = next_path();
    if next.is_file() {
        if let Some(m) = synthetic_manifest(&next).or_else(|| {
            read_manifest(&exe_dir()).map(|mut man| {
                man.file = "vapurr.next.exe".into();
                man
            })
        }) {
            out.push(Candidate {
                source: "next",
                path: next,
                manifest: m,
            });
        }
    }
    if let Some(c) = candidate_from_dir(&channel_dir(), "channel") {
        out.push(c);
    }
    if let Ok(extra) = std::env::var("VAPURR_CHANNEL") {
        let p = PathBuf::from(extra.trim());
        if p.is_dir() {
            if let Some(c) = candidate_from_dir(&p, "env") {
                out.push(c);
            }
        }
    }
    if let Some(dist) = find_repo_dist() {
        if let Some(m) = read_manifest(dist.parent().unwrap_or(Path::new(".")))
            .or_else(|| synthetic_manifest(&dist))
        {
            out.push(Candidate {
                source: "dist",
                path: dist,
                manifest: m,
            });
        }
    }
    out
}

fn running_sha() -> Result<String, String> {
    static HASH: std::sync::OnceLock<Result<String, String>> = std::sync::OnceLock::new();
    HASH.get_or_init(|| sha256_path(&running_exe()?)).clone()
}

fn pick_update(running: &str) -> Option<Candidate> {
    candidates()
        .into_iter()
        .find(|c| !c.manifest.sha256.eq_ignore_ascii_case(running))
}

pub fn status_json() -> serde_json::Value {
    let path = running_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let sha = running_sha().unwrap_or_default();
    let short: String = sha.chars().take(8).collect();
    let found = candidates();
    let update = found
        .iter()
        .find(|c| !c.manifest.sha256.eq_ignore_ascii_case(&sha));
    serde_json::json!({
        "ok": true,
        "running": {
            "version": VERSION,
            "sha256": sha,
            "short": short,
            "path": path,
            "pid": std::process::id(),
        },
        "channel": channel_dir().display().to_string(),
        "has_channel": !found.is_empty(),
        "update": update.map(|c| serde_json::json!({
            "source": c.source,
            "path": c.path.display().to_string(),
            "version": c.manifest.version,
            "build": c.manifest.build,
            "rev": c.manifest.rev,
            "sha256": c.manifest.sha256,
            "short": c.manifest.short_sha(),
            "size": c.manifest.size,
        })),
        "ready": update.is_some(),
    })
}

fn spawn_swap(next: &Path, pid: u32, dest: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        Command::new(next)
            .args([
                "--patch-swap",
                &pid.to_string(),
                dest.to_str().ok_or("dest")?,
            ])
            .creation_flags(BREAKAWAY | DETACH)
            .spawn()
            .map_err(|e| format!("spawn swap: {e}"))?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        Command::new(next)
            .args([
                "--patch-swap",
                &pid.to_string(),
                &dest.display().to_string(),
            ])
            .spawn()
            .map_err(|e| format!("spawn swap: {e}"))?;
        Ok(())
    }
}

/// Stage the channel build as `vapurr.next.exe` and relaunch through `--patch-swap`.
pub fn apply_and_relaunch() -> Result<(), String> {
    let me = running_exe()?;
    let sha = running_sha()?;
    let cand = pick_update(&sha).ok_or_else(|| "already current".to_string())?;
    let dest = dest_exe();
    let next = next_path();
    if cand.path.canonicalize().ok() != next.canonicalize().ok() {
        fs::copy(&cand.path, &next).map_err(|e| format!("stage next: {e}"))?;
    }
    if let Some(parent) = cand.path.parent() {
        if let Some(dir) = dest.parent() {
            copy_if_present(parent, dir, "WebView2Loader.dll");
        }
    }
    let _ = write_manifest(&exe_dir(), &cand.manifest);
    crash::log(&format!(
        "apply {} {} → {}",
        cand.source,
        cand.manifest.short_sha(),
        dest.display()
    ));
    spawn_swap(&next, std::process::id(), &dest)?;
    let _ = me;
    Ok(())
}

fn wait_pid_gone(pid: u32) {
    for _ in 0..80 {
        if !pid_alive(pid) {
            return;
        }
        std::thread::sleep(Duration::from_millis(150));
    }
}

fn pid_alive(pid: u32) -> bool {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::{CloseHandle, WAIT_TIMEOUT};
        use windows::Win32::System::Threading::{
            OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE,
        };
        unsafe {
            let Ok(h) = OpenProcess(PROCESS_SYNCHRONIZE, false, pid) else {
                return false;
            };
            let w = WaitForSingleObject(h, 0);
            let _ = CloseHandle(h);
            w == WAIT_TIMEOUT
        }
    }
    #[cfg(not(windows))]
    {
        let _ = pid;
        false
    }
}

fn copy_retry(src: &Path, dest: &Path) -> Result<(), String> {
    let mut last = String::new();
    for i in 0..40 {
        match fs::copy(src, dest) {
            Ok(_) => return Ok(()),
            Err(e) => {
                last = e.to_string();
                if i == 39 {
                    break;
                }
                std::thread::sleep(Duration::from_millis(150));
            }
        }
    }
    Err(format!("replace {}: {last}", dest.display()))
}

pub fn swap_cli(args: &[String]) -> Result<(), String> {
    let i = args
        .iter()
        .position(|a| a == "--patch-swap")
        .ok_or("missing --patch-swap")?;
    let pid: u32 = args
        .get(i + 1)
        .ok_or("missing pid")?
        .parse()
        .map_err(|_| "bad pid")?;
    let dest = PathBuf::from(args.get(i + 2).ok_or("missing dest")?);
    crash::log(&format!("patch-swap wait pid {pid} → {}", dest.display()));
    wait_pid_gone(pid);
    let src = running_exe()?;
    if let Some(dir) = dest.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    copy_retry(&src, &dest)?;
    if let Some(parent) = src.parent() {
        if let Some(dir) = dest.parent() {
            copy_if_present(parent, dir, "WebView2Loader.dll");
            copy_if_present(parent, dir, "manifest.json");
            copy_if_present(parent, dir, "VERSION.txt");
        }
    }
    crash::log(&format!("patched {}", dest.display()));
    setup_spawn(&dest)?;
    Ok(())
}

fn setup_spawn(exe: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let dir = exe.parent().unwrap_or(Path::new("."));
        Command::new(exe)
            .arg("--app")
            .current_dir(dir)
            .creation_flags(BREAKAWAY | DETACH)
            .spawn()
            .map_err(|e| format!("relaunch: {e}"))?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        Command::new(exe)
            .arg("--app")
            .spawn()
            .map_err(|e| format!("relaunch: {e}"))?;
        Ok(())
    }
}

/// Drop a leftover next.exe that already matches the running binary.
pub fn cleanup_stale_next() {
    let Ok(me) = running_exe() else {
        return;
    };
    let name = me
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if name != "vapurr.exe" {
        return;
    }
    let next = next_path();
    if !next.is_file() {
        return;
    }
    let (Ok(a), Ok(b)) = (sha256_path(&me), sha256_path(&next)) else {
        return;
    };
    if a.eq_ignore_ascii_case(&b) {
        let _ = fs::remove_file(&next);
        crash::log("cleared stale vapurr.next.exe");
    }
}

pub fn handle_cli(args: &[String]) -> bool {
    if args.iter().any(|a| a == "--patch-swap") {
        if let Err(e) = swap_cli(args) {
            crash::log(&format!("patch-swap: {e}"));
            eprintln!("patch-swap: {e}");
        }
        return true;
    }
    if args.iter().any(|a| a == "--publish") {
        if let Err(e) = publish_cli(args) {
            crash::log(&format!("publish: {e}"));
            eprintln!("publish: {e}");
            std::process::exit(1);
        }
        return true;
    }
    if args.iter().any(|a| a == "--patch-status") {
        println!("{}", status_json());
        return true;
    }
    if args.iter().any(|a| a == "--patch-apply") {
        match apply_and_relaunch() {
            Ok(()) => {
                // Parent exits; swap child finishes the replace.
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("patch-apply: {e}");
                std::process::exit(1);
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_empty_file() {
        let dir = std::env::temp_dir().join("vapurr-patch-empty");
        let _ = fs::create_dir_all(&dir);
        let p = dir.join("empty.bin");
        fs::write(&p, b"").unwrap();
        assert_eq!(
            sha256_path(&p).unwrap(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn publish_roundtrip_and_same_hash_is_not_an_update() {
        let root = std::env::temp_dir().join(format!("vapurr-patch-{}", std::process::id()));
        let src_dir = root.join("src");
        let chan = root.join("chan");
        let _ = fs::create_dir_all(&src_dir);
        let src = src_dir.join("vapurr.exe");
        fs::write(&src, b"vapurr-patch-fixture-bytes").unwrap();
        let m = publish_to(&src, &chan, "deadbeef").unwrap();
        assert_eq!(m.rev, "deadbeef");
        assert_eq!(m.version, VERSION);
        assert!(chan.join("vapurr.exe").is_file());
        assert!(chan.join("manifest.json").is_file());
        let again = sha256_path(&chan.join("vapurr.exe")).unwrap();
        assert_eq!(again, m.sha256);
        assert!(m.short_sha().len() == 8);
        let parsed = read_manifest(&chan).unwrap();
        assert_eq!(parsed.sha256, m.sha256);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn pick_skips_matching_sha() {
        let c = Candidate {
            source: "channel",
            path: PathBuf::from("x"),
            manifest: Manifest {
                name: "vapurr".into(),
                version: "0.1.0".into(),
                build: String::new(),
                rev: String::new(),
                sha256: "abc".into(),
                size: 1,
                target: String::new(),
                file: "vapurr.exe".into(),
            },
        };
        assert!(c.manifest.sha256.eq_ignore_ascii_case("ABC"));
        assert!(!c.manifest.sha256.eq_ignore_ascii_case("def"));
    }
}
