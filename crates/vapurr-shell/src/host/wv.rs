use super::*;


pub static FOCUSED: AtomicBool = AtomicBool::new(true);
pub static LIVE_HOME: AtomicBool = AtomicBool::new(true);


pub fn set_focused(on: bool) {
    FOCUSED.store(on, Ordering::Relaxed);
}


pub fn set_live_url(url: &str) {
    LIVE_HOME.store(is_chrome_url(url), Ordering::Relaxed);
}


pub fn is_chrome_url(url: &str) -> bool {
    url.contains("vapurr.localhost") || url.starts_with("vapurr://")
}


/// WalletConnect / mobile-wallet deep links. Robinhood Wallet is a phone app —
/// these must not replace the page (fomo.family) that is showing the QR.
pub fn is_wallet_scheme(url: &str) -> bool {
    let u = url.trim();
    let colon = match u.find(':') {
        Some(i) if i > 0 && i < 24 => i,
        _ => return false,
    };
    let scheme = u[..colon].to_ascii_lowercase();
    matches!(
        scheme.as_str(),
        "wc" | "walletconnect"
            | "ethereum"
            | "solana"
            | "metamask"
            | "rainbow"
            | "cbwallet"
            | "coinbasewallet"
            | "trust"
            | "zerion"
            | "imtokenv2"
            | "phantom"
            | "robinhood"
            | "rhwallet"
    )
}


/// Let WebView2 open a real popup (Privy OAuth, WalletConnect overlay windows).
/// Returning true → wry leaves Handled=false so Edge creates the window.
/// Never navigate the page into the popup URL (`about:blank` would wipe fomo).
pub fn allow_new_window(url: &str) -> bool {
    if is_wallet_scheme(url) {
        return false;
    }
    let u = url.trim();
    u.is_empty()
        || (u.len() >= 11 && u[..11].eq_ignore_ascii_case("about:blank"))
        || u.starts_with("https:")
        || u.starts_with("http:")
        || u.starts_with("blob:")
        || u.starts_with("data:")
}


pub fn want_live_feed() -> bool {
    FOCUSED.load(Ordering::Relaxed) && LIVE_HOME.load(Ordering::Relaxed)
}


/// Keep wry's WebView2 defaults, drop the spare renderer, kill Edge telemetry.
/// Tracking prevention is off so first-party and sign-in cookies actually stick.
/// Does not disable JS, GPU, site isolation, or the cookie jar.
/// Enable-features are Chromium stack that Edge already ships: bfcache, prerender,
/// paint holding, GPU raster, occlusion, timer throttle when occluded.
pub const WV_ARGS: &str = concat!(
    "--disable-features=",
    "msWebOOUI,msPdfOOUI,msSmartScreenProtection,SpareRendererForSitePerProcess,",
    "TrackingPrevention,msEnhancedTrackingPreventionEnabled,",
    "msEnhancedTrackingPreventionContentEnabled,",
    "ThirdPartyCookieDeprecation,TrackingProtection3pcd ",
    "--enable-features=",
    "BackForwardCache,Prerender2,PaintHolding,",
    "CanvasOopRasterization,CalculateNativeWinOcclusion,",
    "IntensiveWakeUpThrottling,PartitionedCookies,FetchPriorityHint,",
    "NavigationPredictor ",
    "--enable-gpu-rasterization ",
    "--enable-zero-copy ",
    "--enable-quic ",
    "--autoplay-policy=no-user-gesture-required ",
    "--disable-sync ",
    "--no-first-run"
);


pub fn wv2_data_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(|h| PathBuf::from(h).join("AppData/Local")))
        .unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("vapurr").join("edge");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

