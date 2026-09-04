use super::*;

#[derive(RustEmbed)]
#[folder = "../../frontend"]
#[exclude = "*.zip"]
#[exclude = "*.mp4"]
pub(crate) struct Frontend;

pub fn frontend_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../frontend")
}

pub(crate) fn mime(path: &str) -> &'static str {
    if path.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if path.ends_with(".js") {
        "application/javascript; charset=utf-8"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".ttf") {
        "font/ttf"
    } else if path.ends_with(".woff2") {
        "font/woff2"
    } else if path.ends_with(".woff") {
        "font/woff"
    } else if path.ends_with(".eot") {
        "application/vnd.ms-fontobject"
    } else if path.ends_with(".otf") {
        "font/otf"
    } else if path.ends_with(".ico") {
        "image/x-icon"
    } else if path.ends_with(".json") {
        "application/json; charset=utf-8"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        "image/jpeg"
    } else if path.ends_with(".webp") {
        "image/webp"
    } else if path.ends_with(".wasm") {
        "application/wasm"
    } else if path.ends_with(".mp4") {
        "video/mp4"
    } else if path.ends_with(".webm") {
        "video/webm"
    } else {
        "text/html; charset=utf-8"
    }
}

pub(crate) fn is_hot_asset(rel: &str) -> bool {
    rel.starts_with("vendor/")
        || rel.starts_with("fonts/")
        || rel.ends_with(".png")
        || rel.ends_with(".jpg")
        || rel.ends_with(".jpeg")
        || rel.ends_with(".webp")
        || rel.ends_with(".svg")
        || rel.ends_with(".woff2")
        || rel.ends_with(".woff")
        || rel.ends_with(".ttf")
        || rel.ends_with(".otf")
        || rel.ends_with(".ico")
        || rel.ends_with(".min.js")
}

pub(crate) fn cache_control(rel: &str) -> &'static str {
    // Posters/trailers iterate overnight; do not pin them immutable.
    if rel.starts_with("ketflix/") {
        "no-store"
    } else if is_hot_asset(rel) {
        "public, max-age=31536000, immutable"
    } else {
        "no-store"
    }
}

/// Live `frontend/` first (dev), then rust-embed (packed). Nested paths ok.
pub(crate) fn read_frontend(rel: &str) -> Option<Cow<'static, [u8]>> {
    let rel = rel.trim_start_matches('/');
    if rel.is_empty() || rel.contains("..") {
        return None;
    }
    let rel = rel.replace('\\', "/");
    let file = frontend_root().join(&rel);
    if let Ok(bytes) = std::fs::read(&file) {
        return Some(Cow::Owned(bytes));
    }
    if let Some(f) = Frontend::get(&rel) {
        return Some(f.data);
    }
    None
}

pub(crate) const HTML_PRELOAD: &str = concat!(
    "</tokens.css>; rel=preload; as=style, ",
    "</chrome.css>; rel=preload; as=style, ",
    "</ipc.js>; rel=preload; as=script, ",
    "</fonts/Sora-Regular.ttf>; rel=preload; as=font; type=font/ttf; crossorigin"
);
