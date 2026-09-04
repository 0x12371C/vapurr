//! Local IPFS-shaped pinset. Ciphertext in, CID out. Optional Kubo / relay.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::cid::{cid_raw_sha256, looks_like_cid};
use crate::ZmailError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PinKind {
    Local,
    Ipfs,
    Relay,
}

pub struct Pinset {
    root: PathBuf,
}

impl Pinset {
    pub fn open(root: impl AsRef<Path>) -> Result<Self, ZmailError> {
        let root = root.as_ref().join("blocks");
        fs::create_dir_all(&root).map_err(|_| ZmailError::Io)?;
        Ok(Self { root })
    }

    pub fn put(&self, bytes: &[u8]) -> Result<(String, PinKind), ZmailError> {
        let cid = cid_raw_sha256(bytes);
        let path = self.block_path(&cid);
        if !path.exists() {
            fs::write(&path, bytes).map_err(|_| ZmailError::Io)?;
        }
        if let Some(got) = try_ipfs_add(&path) {
            if looks_like_cid(&got) {
                return Ok((got, PinKind::Ipfs));
            }
        }
        if try_relay_pin(bytes, &cid) {
            return Ok((cid, PinKind::Relay));
        }
        Ok((cid, PinKind::Local))
    }

    pub fn get(&self, cid: &str) -> Result<Vec<u8>, ZmailError> {
        let cid = cid.trim();
        if !looks_like_cid(cid) {
            return Err(ZmailError::BadCid);
        }
        let path = self.block_path(cid);
        if let Ok(b) = fs::read(&path) {
            if cid_raw_sha256(&b) == cid {
                return Ok(b);
            }
        }
        if let Some(b) = fetch_gateway(cid) {
            let _ = fs::write(&path, &b);
            return Ok(b);
        }
        Err(ZmailError::Missing)
    }

    pub fn list(&self) -> Vec<String> {
        let mut out = Vec::new();
        let Ok(rd) = fs::read_dir(&self.root) else {
            return out;
        };
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if looks_like_cid(&n) {
                out.push(n);
            }
        }
        out
    }

    fn block_path(&self, cid: &str) -> PathBuf {
        self.root.join(cid)
    }
}

pub fn relay_notify_inbox(to: &str, cid: &str) {
    let Ok(base) = std::env::var("VAPURR_MAIL_RELAY") else {
        return;
    };
    let base = base.trim().trim_end_matches('/');
    if base.is_empty() {
        return;
    }
    let url = format!("{base}/inbox");
    let body = serde_json::json!({ "to": to, "cid": cid });
    let _ = post_json(&url, &body);
}

pub fn relay_list_inbox(to: &str) -> Vec<String> {
    let Ok(base) = std::env::var("VAPURR_MAIL_RELAY") else {
        return Vec::new();
    };
    let base = base.trim().trim_end_matches('/');
    if base.is_empty() {
        return Vec::new();
    }
    let enc = to.trim().trim_start_matches('@');
    let url = format!("{base}/inbox/{enc}");
    let Some(txt) = get_text(&url) else {
        return Vec::new();
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&txt) else {
        return Vec::new();
    };
    v.get("cids")
        .and_then(|c| c.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str())
                .filter(|s| looks_like_cid(s))
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn try_ipfs_add(path: &Path) -> Option<String> {
    let mut cmd = Command::new("ipfs");
    cmd.args(["add", "-Q", "--cid-version=1", "--raw-leaves", "--pin=true"])
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = cmd.spawn().ok()?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(st)) if st.success() => {
                let mut s = String::new();
                if let Some(mut out) = child.stdout.take() {
                    let _ = out.read_to_string(&mut s);
                }
                let cid = s.trim();
                if looks_like_cid(cid) {
                    return Some(cid.to_string());
                }
                return None;
            }
            Ok(Some(_)) => return None,
            Ok(None) if start.elapsed() > Duration::from_secs(3) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(40)),
            Err(_) => return None,
        }
    }
}

fn try_relay_pin(bytes: &[u8], cid: &str) -> bool {
    let url = match std::env::var("VAPURR_MAIL_RELAY") {
        Ok(u) if !u.trim().is_empty() => format!("{}/pin", u.trim().trim_end_matches('/')),
        _ => return false,
    };
    let body = serde_json::json!({
        "cid": cid,
        "bytes": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes),
    });
    post_json(&url, &body).is_some()
}

fn fetch_gateway(cid: &str) -> Option<Vec<u8>> {
    if let Ok(base) = std::env::var("VAPURR_MAIL_RELAY") {
        let url = format!("{}/ipfs/{}", base.trim().trim_end_matches('/'), cid);
        if let Some(b) = get_bytes(&url) {
            if cid_raw_sha256(&b) == cid {
                return Some(b);
            }
        }
    }
    let gates = [
        format!("https://ipfs.io/ipfs/{cid}"),
        format!("https://cloudflare-ipfs.com/ipfs/{cid}"),
        format!("https://w3s.link/ipfs/{cid}"),
    ];
    for url in gates {
        if let Some(b) = get_bytes(&url) {
            if cid_raw_sha256(&b) == cid {
                return Some(b);
            }
        }
    }
    None
}

fn client() -> Option<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .ok()
}

fn post_json(url: &str, body: &serde_json::Value) -> Option<String> {
    let c = client()?;
    let r = c.post(url).json(body).send().ok()?;
    if !r.status().is_success() {
        return None;
    }
    r.text().ok()
}

fn get_bytes(url: &str) -> Option<Vec<u8>> {
    let c = client()?;
    let r = c.get(url).send().ok()?;
    if !r.status().is_success() {
        return None;
    }
    r.bytes().ok().map(|b| b.to_vec())
}

fn get_text(url: &str) -> Option<String> {
    let c = client()?;
    let r = c.get(url).send().ok()?;
    if !r.status().is_success() {
        return None;
    }
    r.text().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{rngs::OsRng, RngCore};

    fn tmp() -> PathBuf {
        let mut n = [0u8; 8];
        OsRng.fill_bytes(&mut n);
        std::env::temp_dir().join(format!("zzzmail-pin-{}-{}", std::process::id(), hex::encode(n)))
    }

    #[test]
    fn put_get_roundtrip() {
        let dir = tmp();
        let p = Pinset::open(&dir).unwrap();
        let (cid, kind) = p.put(b"sealed-letter").unwrap();
        assert!(matches!(kind, PinKind::Local | PinKind::Ipfs | PinKind::Relay));
        assert!(cid.starts_with("bafkrei"));
        assert_eq!(p.get(&cid).unwrap(), b"sealed-letter");
        let _ = fs::remove_dir_all(dir);
    }
}
