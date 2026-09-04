use super::*;


#[derive(RustEmbed)]
#[folder = "../../frontend"]
#[exclude = "*.zip"]
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
    if is_hot_asset(rel) {
        "public, max-age=31536000, immutable"
    } else {
        "no-store"
    }
}


pub(crate) const HTML_PRELOAD: &str = concat!(
    "</tokens.css>; rel=preload; as=style, ",
    "</chrome.css>; rel=preload; as=style, ",
    "</ipc.js>; rel=preload; as=script, ",
    "</fonts/Sora-Regular.ttf>; rel=preload; as=font; type=font/ttf; crossorigin"
);
