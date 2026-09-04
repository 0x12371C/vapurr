//! VAPURR window: chrome + WebView2 guest + local earn desk.
#![windows_subsystem = "windows"]

mod cookies;
mod crash;
mod desk;
mod host;
mod inject;
mod ipc;
mod layout;
mod nav;
mod patch;
mod setup;
mod shield_hook;
mod tabs;
use inject::*;
use ipc::*;
use layout::*;
use nav::*;

#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use tao::dpi::{LogicalSize, PhysicalSize};
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::{Icon, Theme, WindowBuilder};

#[cfg(windows)]
use tao::platform::windows::{
    EventLoopBuilderExtWindows, IconExtWindows, WindowBuilderExtWindows, WindowExtWindows,
};

/// Paint the native Windows caption to match vapurr void / light paper.
#[cfg(windows)]
fn paint_caption(window: &tao::window::Window, light: bool) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR,
        DWMWA_USE_IMMERSIVE_DARK_MODE,
    };

    let hwnd = HWND(window.hwnd() as *mut _);
    // COLORREF is 0x00BBGGRR.
    let caption: u32 = if light { 0x00F0_F5F3 } else { 0x000E_0E0E };
    let text: u32 = if light { 0x0016_1816 } else { 0x00F4_F3F2 };
    let dark = windows::Win32::Foundation::BOOL::from(!light);
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            &dark as *const _ as *const core::ffi::c_void,
            std::mem::size_of_val(&dark) as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_CAPTION_COLOR,
            &caption as *const _ as *const core::ffi::c_void,
            std::mem::size_of_val(&caption) as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_BORDER_COLOR,
            &caption as *const _ as *const core::ffi::c_void,
            std::mem::size_of_val(&caption) as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_TEXT_COLOR,
            &text as *const _ as *const core::ffi::c_void,
            std::mem::size_of_val(&text) as u32,
        );
    }
}

fn app_icon(px: u32) -> Option<Icon> {
    #[cfg(windows)]
    {
        Icon::from_resource(1, Some(PhysicalSize::new(px, px))).ok()
    }
    #[cfg(not(windows))]
    {
        let _ = px;
        None
    }
}
use wry::{PageLoadEvent, WebContext, WebViewBuilder};

#[cfg(windows)]
use wry::{MemoryUsageLevel, WebViewBuilderExtWindows, WebViewExtWindows};

use crate::desk::Desk;
use crate::host::serve;
use crate::tabs::TabStrip;
fn desk_json(desk: &Desk, shield: &vapurr_shield::Shield) -> serde_json::Value {
    let mut v = desk.snapshot();
    if let Some(map) = v.as_object_mut() {
        map.insert("shield".into(), shield.snapshot());
        if let Some(id) = setup::read_install_id() {
            map.insert("install_id".into(), serde_json::Value::String(id));
        }
    }
    v
}
fn downloads_dir() -> PathBuf {
    std::env::var("USERPROFILE")
        .map(|h| PathBuf::from(h).join("Downloads"))
        .unwrap_or_else(|_| PathBuf::from("."))
}

fn filename_from_url(url: &str) -> String {
    let noq = url.split('?').next().unwrap_or(url);
    let name = noq.rsplit('/').next().unwrap_or("download");
    if name.is_empty() {
        "download".into()
    } else {
        name.chars().take(80).collect()
    }
}

fn fatal(where_: &str, e: impl std::fmt::Display) -> ! {
    let msg = format!("{where_}: {e}");
    crash::log(&msg);
    #[cfg(windows)]
    {
        use windows::core::HSTRING;
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
        let body = if msg.contains("WebView")
            || where_ == "sidebar"
            || where_ == "toolbar"
            || where_ == "page"
            || where_ == "radio"
        {
            format!(
                "{msg}\n\nNeed Microsoft Edge WebView2 Runtime.\nhttps://go.microsoft.com/fwlink/p/?LinkId=2124703"
            )
        } else {
            msg.clone()
        };
        let title = HSTRING::from("vapurr");
        let text = HSTRING::from(body.as_str());
        unsafe {
            let _ = MessageBoxW(
                HWND(std::ptr::null_mut()),
                &text,
                &title,
                MB_OK | MB_ICONERROR,
            );
        }
    }
    std::process::exit(1);
}

fn set_dpi() {
    #[cfg(windows)]
    {
        use windows::Win32::UI::HiDpi::{
            SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        };
        let _ =
            unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    }
}

fn main() {
    set_dpi();
    crash::install();
    let args: Vec<String> = std::env::args().collect();
    if patch::handle_cli(&args) {
        return;
    }
    if args.iter().any(|a| a == "--uninstall") {
        setup::uninstall_silent();
        return;
    }
    if args.iter().any(|a| a == "--install") {
        let desktop = !args.iter().any(|a| a == "--no-desktop");
        if let Err(e) = setup::install_now(desktop) {
            fatal("install", e);
        }
        return;
    }
    if setup::wants_setup(&args) {
        setup::run();
        return;
    }
    setup::set_aumid("vapurr.Desktop");
    crash::log("vapurr starting");
    patch::cleanup_stale_next();
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(crash::log_path())
        .ok();
    let subscriber = tracing_subscriber::fmt().with_env_filter("vapurr=info");
    if let Some(file) = log_file {
        subscriber.with_writer(std::sync::Mutex::new(file)).init();
    } else {
        subscriber.init();
    }

    let mut event_loop_b = EventLoopBuilder::<Msg>::with_user_event();
    #[cfg(windows)]
    {
        event_loop_b.with_theme(Some(Theme::Dark));
    }
    let event_loop = event_loop_b.build();
    let proxy = event_loop.create_proxy();
    let mut window_b = WindowBuilder::new()
        .with_title("VAPURR")
        .with_theme(Some(Theme::Dark))
        .with_inner_size(LogicalSize::new(1280.0, 800.0))
        .with_min_inner_size(LogicalSize::new(900.0, 560.0))
        .with_window_icon(app_icon(32));
    #[cfg(windows)]
    {
        window_b = window_b
            .with_taskbar_icon(app_icon(256))
            .with_window_classname("VAPURR");
    }
    let window = window_b
        .build(&event_loop)
        .unwrap_or_else(|e| fatal("window", e));
    #[cfg(windows)]
    paint_caption(&window, false);
    crash::log("window created");

    let size = window.inner_size();
    let sf = window.scale_factor();
    let log: LogicalSize<f64> = PhysicalSize::new(size.width, size.height).to_logical(sf);
    let radio_ui0 = RadioUi::default();
    let (side_r, bar_r, page_r, radio_r) = layout(log.width, log.height, &radio_ui0);

    let ipc = {
        let proxy = proxy.clone();
        move |req: wry::http::Request<String>| {
            if let Some(msg) = parse_ipc(req.body()) {
                let _ = proxy.send_event(msg);
            }
        }
    };

    let mut web_ctx = WebContext::new(Some(host::wv2_data_dir()));

    let mut sidebar_b = WebViewBuilder::with_web_context(&mut web_ctx)
        .with_bounds(side_r)
        .with_background_color((0x0E, 0x0E, 0x0E, 255))
        .with_custom_protocol("vapurr".into(), serve)
        .with_initialization_script(BOOT_JS)
        .with_ipc_handler(ipc.clone())
        .with_url(vapurr_url("sidebar.html"));
    #[cfg(windows)]
    {
        sidebar_b = sidebar_b.with_additional_browser_args(host::WV_ARGS);
    }
    let sidebar = sidebar_b
        .build_as_child(&window)
        .unwrap_or_else(|e| fatal("sidebar", e));
    crash::log("sidebar ok");

    let mut toolbar_b = WebViewBuilder::with_web_context(&mut web_ctx)
        .with_bounds(bar_r)
        .with_background_color((0x0E, 0x0E, 0x0E, 255))
        .with_custom_protocol("vapurr".into(), serve)
        .with_initialization_script(BOOT_JS)
        .with_ipc_handler(ipc.clone())
        .with_url(vapurr_url("toolbar.html"));
    #[cfg(windows)]
    {
        toolbar_b = toolbar_b.with_additional_browser_args(host::WV_ARGS);
    }
    let toolbar = toolbar_b
        .build_as_child(&window)
        .unwrap_or_else(|e| fatal("toolbar", e));
    crash::log("toolbar ok");

    let page_nav = {
        let proxy = proxy.clone();
        move |url: String| {
            // Stay on fomo.family when Connect emits a wc: deep link.
            if host::is_wallet_scheme(&url) {
                return false;
            }
            let fixed = canonicalize_url(&url);
            if fixed != url {
                let _ = proxy.send_event(Msg::Go(fixed));
                return false;
            }
            let _ = proxy.send_event(Msg::Url(url));
            true
        }
    };
    let page_new = { move |url: String| host::allow_new_window(&url) };
    let page_load = {
        let proxy = proxy.clone();
        move |ev: PageLoadEvent, url: String| match ev {
            PageLoadEvent::Started => {
                let _ = proxy.send_event(Msg::PageStart(url));
            }
            PageLoadEvent::Finished => {
                let _ = proxy.send_event(Msg::Url(url));
            }
        }
    };
    let page_title = {
        let proxy = proxy.clone();
        move |title: String| {
            let _ = proxy.send_event(Msg::Title(title));
        }
    };
    let dl_dir = downloads_dir();

    let mut page_b = WebViewBuilder::with_web_context(&mut web_ctx)
        .with_bounds(page_r)
        .with_background_color((0x0E, 0x0E, 0x0E, 255))
        .with_custom_protocol("vapurr".into(), serve)
        .with_initialization_script(BOOT_JS)
        .with_ipc_handler(ipc.clone())
        .with_navigation_handler(page_nav)
        .with_new_window_req_handler(page_new)
        .with_on_page_load_handler(page_load)
        .with_document_title_changed_handler(page_title)
        .with_download_started_handler(move |url, path| {
            let dest = dl_dir.join(filename_from_url(&url));
            *path = dest;
            true
        })
        .with_clipboard(true)
        .with_hotkeys_zoom(false)
        .with_devtools(cfg!(debug_assertions))
        .with_focused(true)
        .with_url(vapurr_url("home.html?v=wordmark"));
    #[cfg(windows)]
    {
        page_b = page_b.with_additional_browser_args(host::WV_ARGS);
    }
    let page = page_b
        .build_as_child(&window)
        .unwrap_or_else(|e| fatal("page", e));
    crash::log("page ok");

    let mut radio_b = WebViewBuilder::with_web_context(&mut web_ctx)
        .with_bounds(radio_r)
        .with_background_color((0x03, 0x03, 0x03, 255))
        .with_custom_protocol("vapurr".into(), serve)
        .with_initialization_script(BOOT_JS)
        .with_ipc_handler(ipc)
        .with_url(vapurr_url("radio.html"));
    #[cfg(windows)]
    {
        radio_b = radio_b.with_additional_browser_args(host::WV_ARGS);
    }
    let radio = radio_b
        .build_as_child(&window)
        .unwrap_or_else(|e| fatal("radio", e));
    crash::log("radio ok, entering loop");

    let sidebar = Rc::new(RefCell::new(sidebar));
    let toolbar = Rc::new(RefCell::new(toolbar));
    let page = Rc::new(RefCell::new(page));
    let radio = Rc::new(RefCell::new(radio));
    let radio_ui = Rc::new(RefCell::new(radio_ui0));
    cookies::ensure_jar(&page.borrow());
    let _ = std::thread::Builder::new()
        .name("desk-warm".into())
        .spawn(|| {
            vapurr_rhc::scan::warm();
            vapurr_fomo::warm();
            let _ = vapurr_rhc::route::tokens_json("");
        });
    let last_chain = Rc::new(RefCell::new(String::new()));
    let desk = Rc::new(RefCell::new(Desk::load()));
    let vault = Rc::new(RefCell::new(
        vapurr_blob::Vault::open_default().unwrap_or_else(|e| {
            crash::log(&format!("blob vault: {e}"));
            vapurr_blob::Vault::open(std::env::temp_dir().join("vapurr-blobs")).expect("blob vault")
        }),
    ));
    snap_desk(&desk.borrow(), &vault);
    {
        let light = desk.borrow().prefs.theme == "light";
        #[cfg(windows)]
        paint_caption(&window, light);
        window.set_theme(Some(if light { Theme::Light } else { Theme::Dark }));
    }
    let last_econ = Rc::new(RefCell::new(serde_json::json!({})));
    let last_outbid = Rc::new(RefCell::new(serde_json::json!({})));
    let last_ketlist = Rc::new(RefCell::new(serde_json::json!({})));
    let last_wallet = Rc::new(RefCell::new(serde_json::json!({})));
    let (econ_tx, econ_rx) = mpsc::channel::<vapurr_econ::EconCmd>();
    {
        let proxy = proxy.clone();
        std::thread::Builder::new()
            .name("vapurr-econ".into())
            .spawn(move || {
                let mut client = vapurr_econ::Client::open();
                let snap = client.snapshot();
                let _ = proxy.send_event(Msg::EconSnap(snap));
                while let Ok(cmd) = econ_rx.recv() {
                    let board = matches!(
                        &cmd,
                        vapurr_econ::EconCmd::Outbid
                            | vapurr_econ::EconCmd::OutbidBid { .. }
                            | vapurr_econ::EconCmd::OutbidDeploy
                    );
                    let listed = matches!(
                        &cmd,
                        vapurr_econ::EconCmd::KetList
                            | vapurr_econ::EconCmd::KetListPay { .. }
                            | vapurr_econ::EconCmd::KetListDeploy
                    );
                    match client.run(cmd) {
                        Ok(snap) if board => {
                            let _ = proxy.send_event(Msg::OutbidSnap(snap));
                        }
                        Ok(snap) if listed => {
                            let _ = proxy.send_event(Msg::KetListSnap(snap));
                        }
                        Ok(snap) => {
                            let _ = proxy.send_event(Msg::EconSnap(snap));
                        }
                        Err(fail) => {
                            let _ = proxy.send_event(Msg::EconErr {
                                which: fail.which,
                                msg: fail.msg,
                            });
                        }
                    }
                }
            })
            .ok();
    }
    let (wallet_tx, wallet_rx) = mpsc::channel::<vapurr_wallet::WalletCmd>();
    {
        let proxy = proxy.clone();
        std::thread::Builder::new()
            .name("vapurr-wallet".into())
            .spawn(move || {
                let mut client = vapurr_wallet::WalletDesk::open();
                let snap = client.snap();
                let _ = proxy.send_event(Msg::WalletSnap(snap));
                while let Ok(cmd) = wallet_rx.recv() {
                    match client.run(cmd) {
                        Ok(snap) => {
                            let _ = proxy.send_event(Msg::WalletSnap(snap));
                        }
                        Err(e) => {
                            let _ = proxy.send_event(Msg::WalletErr(e.to_string()));
                        }
                    }
                }
            })
            .ok();
    }
    let start_url = {
        let d = desk.borrow();
        if d.prefs.restore_last && !d.last_url.is_empty() {
            d.last_url.clone()
        } else {
            home_url(&d)
        }
    };
    let tabs = Rc::new(RefCell::new(TabStrip::new(start_url.clone())));
    let zoom = Rc::new(RefCell::new(clamp_zoom(desk.borrow().prefs.zoom)));
    let shield = {
        let d = desk.borrow();
        vapurr_shield::Shield::with_prefs(vapurr_shield::ShieldPrefs {
            enabled: d.prefs.adblock,
            privacy: d.prefs.adblock_privacy,
            annoyances: d.prefs.adblock_annoyances,
            cosmetic: d.prefs.adblock_cosmetic,
        })
    };
    let page_url = Arc::new(Mutex::new(start_url.clone()));
    shield_hook::attach(&page.borrow(), shield.clone(), page_url.clone());
    shield_hook::attach_wallet_schemes(&page.borrow());
    if start_url != vapurr_url("home.html")
        && start_url != vapurr_url("home.html?v=cat")
        && start_url != vapurr_url("home.html?v=wordmark")
    {
        tabs.borrow_mut().suppress = true;
        let _ = page.borrow().load_url(&start_url);
    }
    let _ = page.borrow().zoom(*zoom.borrow());

    {
        let proxy = proxy.clone();
        std::thread::Builder::new()
            .name("rhc-rpc".into())
            .spawn(move || {
                let mut rpc = vapurr_rhc::Rpc::new();
                let mut last_err = std::time::Instant::now()
                    .checked_sub(std::time::Duration::from_secs(30))
                    .unwrap_or_else(std::time::Instant::now);
                loop {
                    let live = host::want_live_feed();
                    match rpc.poll(false) {
                        Ok(Some(feed)) => {
                            if let Ok(json) = serde_json::to_string(&feed) {
                                let _ = proxy.send_event(Msg::Chain(json));
                            }
                        }
                        Ok(None) => {}
                        Err(e) => {
                            if last_err.elapsed() > std::time::Duration::from_secs(8) {
                                crash::log(&format!("rhc rpc: {e}"));
                                last_err = std::time::Instant::now();
                            }
                        }
                    }
                    let ms = if live { 2000 } else { 5000 };
                    std::thread::sleep(std::time::Duration::from_millis(ms));
                }
            })
            .ok();
    }

    let paint_zoom = {
        let page = page.clone();
        let toolbar = toolbar.clone();
        let sidebar = sidebar.clone();
        let zoom = zoom.clone();
        let desk = desk.clone();
        move || {
            let z = *zoom.borrow();
            let _ = page.borrow().zoom(z);
            let d = desk.borrow();
            let js = js_set_zoom(
                (z * 100.0).round() as i64,
                d.prefs.show_zoom_chip,
                d.prefs.ctrl_scroll_zoom,
            );
            let _ = toolbar.borrow().evaluate_script(&js);
            let _ = page.borrow().evaluate_script(&js);
            let _ = sidebar.borrow().evaluate_script(&js);
        }
    };

    let paint_chrome = {
        let toolbar = toolbar.clone();
        let sidebar = sidebar.clone();
        let page = page.clone();
        let radio = radio.clone();
        let tabs = tabs.clone();
        let desk = desk.clone();
        let shield = shield.clone();
        let last_econ = last_econ.clone();
        let last_outbid = last_outbid.clone();
        let last_ketlist = last_ketlist.clone();
        let last_wallet = last_wallet.clone();
        let paint_zoom = paint_zoom.clone();
        move || {
            let tjson = tabs.borrow().json();
            let _ = toolbar.borrow().evaluate_script(&js_set_tabs(&tjson));
            let url = tabs.borrow().current().url().to_string();
            host::set_live_url(&url);
            let _ = toolbar.borrow().evaluate_script(&js_set_url(&url));
            let starred = desk.borrow().is_starred(&url);
            let _ = toolbar.borrow().evaluate_script(&format!(
                "window.__setStar && window.__setStar({})",
                if starred { "true" } else { "false" }
            ));
            let djson = desk_json(&desk.borrow(), &shield);
            let _ = page.borrow().evaluate_script(&js_set_desk(&djson));
            let boost_on = desk.borrow().prefs.boost;
            let bjs = js_set_boost(boost_on);
            let _ = page.borrow().evaluate_script(&bjs);
            let _ = toolbar.borrow().evaluate_script(&bjs);
            let theme = desk.borrow().prefs.theme.clone();
            let tjs = js_apply_theme(&theme);
            let _ = sidebar.borrow().evaluate_script(&tjs);
            let _ = toolbar.borrow().evaluate_script(&tjs);
            let _ = radio.borrow().evaluate_script(&tjs);
            if host::is_chrome_url(&url) {
                let _ = page.borrow().evaluate_script(&tjs);
                let ej = last_econ.borrow().clone();
                if ej.is_object() && !ej.as_object().map(|o| o.is_empty()).unwrap_or(true) {
                    let _ = page.borrow().evaluate_script(&js_set_econ(&ej));
                }
                let oj = last_outbid.borrow().clone();
                if oj.is_object() && !oj.as_object().map(|o| o.is_empty()).unwrap_or(true) {
                    let _ = page.borrow().evaluate_script(&js_set_outbid(&oj));
                }
                let kj = last_ketlist.borrow().clone();
                if kj.is_object() && !kj.as_object().map(|o| o.is_empty()).unwrap_or(true) {
                    let _ = page.borrow().evaluate_script(&js_set_ketlist(&kj));
                }
                let wj = last_wallet.borrow().clone();
                if wj.is_object() && !wj.as_object().map(|o| o.is_empty()).unwrap_or(true) {
                    let _ = page.borrow().evaluate_script(&js_set_wallet(&wj));
                }
            }
            paint_zoom();
        }
    };

    paint_chrome();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::UserEvent(Msg::Go(url)) => {
                let url = resolve_nav(&url);
                tabs.borrow_mut().navigate(url.clone());
                tabs.borrow_mut().suppress = true;
                set_page_url(&page_url, &url);
                let _ = page.borrow().load_url(&url);
                paint_chrome();
            }
            Event::UserEvent(Msg::Home) => {
                let url = home_url(&desk.borrow());
                tabs.borrow_mut().navigate(url.clone());
                tabs.borrow_mut().suppress = true;
                set_page_url(&page_url, &url);
                let _ = page.borrow().load_url(&url);
                paint_chrome();
            }
            Event::UserEvent(Msg::Back) => {
                // Drop the RefMut before the next borrow — 2021 if-let keeps scrutinee temps alive.
                let url = tabs.borrow_mut().back();
                if let Some(url) = url {
                    tabs.borrow_mut().suppress = true;
                    set_page_url(&page_url, &url);
                    let _ = page.borrow().load_url(&url);
                    paint_chrome();
                }
            }
            Event::UserEvent(Msg::Forward) => {
                let url = tabs.borrow_mut().forward();
                if let Some(url) = url {
                    tabs.borrow_mut().suppress = true;
                    set_page_url(&page_url, &url);
                    let _ = page.borrow().load_url(&url);
                    paint_chrome();
                }
            }
            Event::UserEvent(Msg::Reload) => {
                let _ = page.borrow().evaluate_script("location.reload()");
            }
            Event::UserEvent(Msg::Pane(id)) => {
                let cur = tabs.borrow().current().url().to_string();
                let want = pane_url(&id);
                let same = match id.as_str() {
                    "settings" => cur.contains("settings.html"),
                    "id" => cur.contains("id.html"),
                    "shield" | "adblock" => cur.contains("shield.html"),
                    _ => {
                        let stem = want.rsplit('/').next().unwrap_or("");
                        !stem.is_empty() && cur.contains(stem)
                    }
                };
                let url = if same { home_url(&desk.borrow()) } else { want };
                tabs.borrow_mut().navigate(url.clone());
                tabs.borrow_mut().suppress = true;
                set_page_url(&page_url, &url);
                let _ = page.borrow().load_url(&url);
                paint_chrome();
            }
            Event::UserEvent(Msg::NewTab) => {
                let url = home_url(&desk.borrow());
                let url = tabs.borrow_mut().new_tab(url);
                tabs.borrow_mut().suppress = true;
                set_page_url(&page_url, &url);
                let _ = page.borrow().load_url(&url);
                paint_chrome();
            }
            Event::UserEvent(Msg::CloseTab(id)) => {
                let home = home_url(&desk.borrow());
                let url = tabs.borrow_mut().close(id, &home);
                if let Some(url) = url {
                    tabs.borrow_mut().suppress = true;
                    set_page_url(&page_url, &url);
                    let _ = page.borrow().load_url(&url);
                }
                paint_chrome();
            }
            Event::UserEvent(Msg::SelectTab(id)) => {
                let url = tabs.borrow_mut().select(id);
                if let Some(url) = url {
                    tabs.borrow_mut().suppress = true;
                    set_page_url(&page_url, &url);
                    let _ = page.borrow().load_url(&url);
                }
                paint_chrome();
            }
            Event::UserEvent(Msg::SelectTabAt(i)) => {
                let url = tabs.borrow_mut().select_at(i as usize);
                if let Some(url) = url {
                    tabs.borrow_mut().suppress = true;
                    set_page_url(&page_url, &url);
                    let _ = page.borrow().load_url(&url);
                }
                paint_chrome();
            }
            Event::UserEvent(Msg::CycleTab { back }) => {
                let url = tabs.borrow_mut().cycle(back);
                if let Some(url) = url {
                    tabs.borrow_mut().suppress = true;
                    set_page_url(&page_url, &url);
                    let _ = page.borrow().load_url(&url);
                }
                paint_chrome();
            }
            Event::UserEvent(Msg::Star { url, title }) => {
                let (url, title) = match url {
                    Some(u) if !u.is_empty() => (u, title.unwrap_or_default()),
                    _ => {
                        let t = tabs.borrow();
                        (t.current().url().to_string(), t.current().title.clone())
                    }
                };
                let on = desk.borrow_mut().toggle_star(&url, &title);
                snap_desk(&desk.borrow(), &vault);
                let _ = toolbar.borrow().evaluate_script(&format!(
                    "window.__setStar && window.__setStar({})",
                    if on { "true" } else { "false" }
                ));
                paint_chrome();
            }
            Event::UserEvent(Msg::Cookies) => {
                let url = tabs.borrow().current().url().to_string();
                cookies::push(&page.borrow(), &url);
            }
            Event::UserEvent(Msg::CookieDel {
                name,
                domain,
                path,
                host,
            }) => {
                if let Some(host) = host.filter(|h| !h.is_empty()) {
                    let _ = cookies::delete_host(&page.borrow(), &host);
                } else if name.is_empty() && domain.is_empty() {
                    let _ = cookies::delete_all(&page.borrow());
                    cookies::ensure_jar(&page.borrow());
                } else {
                    let _ = cookies::delete_one(&page.borrow(), &name, &domain, &path);
                }
                let url = tabs.borrow().current().url().to_string();
                cookies::push(&page.borrow(), &url);
            }
            Event::UserEvent(Msg::ShowFind) => {
                let _ = toolbar
                    .borrow()
                    .evaluate_script("window.__showFind && window.__showFind()");
            }
            Event::UserEvent(Msg::Find(q)) => {
                if !q.is_empty() {
                    let js = format!(
                        "window.find({}, false, false, true, false, false, false)",
                        serde_json::to_string(&q).unwrap_or_else(|_| "\"\"".into())
                    );
                    let _ = page.borrow().evaluate_script(&js);
                }
            }
            Event::UserEvent(Msg::FocusUrl) => {
                let _ = toolbar
                    .borrow()
                    .evaluate_script("window.__focusUrl && window.__focusUrl()");
            }
            Event::UserEvent(Msg::ZoomIn) => {
                let z = zoom_in(*zoom.borrow());
                *zoom.borrow_mut() = z;
                desk.borrow_mut().set_zoom(z);
                paint_zoom();
            }
            Event::UserEvent(Msg::ZoomOut) => {
                let z = zoom_out(*zoom.borrow());
                *zoom.borrow_mut() = z;
                desk.borrow_mut().set_zoom(z);
                paint_zoom();
            }
            Event::UserEvent(Msg::ZoomReset) => {
                *zoom.borrow_mut() = 1.0;
                desk.borrow_mut().set_zoom(1.0);
                paint_zoom();
            }
            Event::UserEvent(Msg::ZoomSet(factor)) => {
                let z = clamp_zoom(factor);
                *zoom.borrow_mut() = z;
                desk.borrow_mut().set_zoom(z);
                paint_zoom();
                paint_chrome();
            }
            Event::UserEvent(Msg::Pref(key, value)) => {
                let on = value.as_bool().unwrap_or(true);
                match key.as_str() {
                    "homepage" => {
                        if let Some(s) = value.as_str() {
                            desk.borrow_mut().set_homepage(s);
                        }
                    }
                    "ctrl_scroll_zoom" => {
                        desk.borrow_mut().set_ctrl_scroll_zoom(on);
                    }
                    "show_zoom_chip" => {
                        desk.borrow_mut().set_show_zoom_chip(on);
                    }
                    "restore_last" => {
                        desk.borrow_mut()
                            .set_restore_last(value.as_bool().unwrap_or(false));
                    }
                    "boost" => {
                        desk.borrow_mut().set_boost(on);
                    }
                    "theme" => {
                        let t = value.as_str().unwrap_or(if on { "light" } else { "dark" });
                        desk.borrow_mut().set_theme(t);
                        let light = desk.borrow().prefs.theme == "light";
                        #[cfg(windows)]
                        paint_caption(&window, light);
                        window.set_theme(Some(if light { Theme::Light } else { Theme::Dark }));
                    }
                    "adblock" | "adblock_privacy" | "adblock_annoyances" | "adblock_cosmetic" => {
                        let old = shield.prefs();
                        match key.as_str() {
                            "adblock" => desk.borrow_mut().set_adblock(on),
                            "adblock_privacy" => desk.borrow_mut().set_adblock_privacy(on),
                            "adblock_annoyances" => desk.borrow_mut().set_adblock_annoyances(on),
                            "adblock_cosmetic" => desk.borrow_mut().set_adblock_cosmetic(on),
                            _ => {}
                        }
                        let d = desk.borrow();
                        let new = vapurr_shield::ShieldPrefs {
                            enabled: d.prefs.adblock,
                            privacy: d.prefs.adblock_privacy,
                            annoyances: d.prefs.adblock_annoyances,
                            cosmetic: d.prefs.adblock_cosmetic,
                        };
                        drop(d);
                        shield.set_prefs(new);
                        if new.enabled
                            && (old.privacy != new.privacy
                                || old.annoyances != new.annoyances
                                || !old.enabled)
                        {
                            shield.refresh_remote();
                        }
                    }
                    _ => {}
                }
                snap_desk(&desk.borrow(), &vault);
                paint_chrome();
            }
            Event::UserEvent(Msg::Clear(what)) => {
                match what.as_str() {
                    "history" => desk.borrow_mut().clear_history(),
                    "bookmarks" => desk.borrow_mut().clear_bookmarks(),
                    "visits" => desk.borrow_mut().clear_visits(),
                    "cookies" => {
                        let _ = cookies::delete_all(&page.borrow());
                        cookies::ensure_jar(&page.borrow());
                    }
                    "all" => {
                        desk.borrow_mut().clear_history();
                        desk.borrow_mut().clear_bookmarks();
                        desk.borrow_mut().clear_visits();
                        let _ = cookies::delete_all(&page.borrow());
                        cookies::ensure_jar(&page.borrow());
                    }
                    _ => {}
                }
                snap_desk(&desk.borrow(), &vault);
                paint_chrome();
                let url = tabs.borrow().current().url().to_string();
                if url.contains("cookies.html") {
                    cookies::push(&page.borrow(), &url);
                }
            }
            Event::UserEvent(Msg::EarnToggle) => {
                let on = !desk.borrow().opt_in;
                desk.borrow_mut().set_opt_in(on);
                snap_desk(&desk.borrow(), &vault);
                paint_chrome();
            }
            Event::UserEvent(Msg::EarnSubmit) => {
                let proven = vapurr_id::load_verified(&Desk::profile_dir())
                    .as_ref()
                    .map(vapurr_id::payout_ready)
                    .unwrap_or(false);
                let _ = desk.borrow_mut().submit(proven);
                snap_desk(&desk.borrow(), &vault);
                paint_chrome();
            }
            Event::UserEvent(Msg::Blobs) => {
                let snap = vault.borrow().snapshot();
                let _ = page.borrow().evaluate_script(&js_set_blobs(&snap));
            }
            Event::UserEvent(Msg::BlobSnap) => {
                snap_desk(&desk.borrow(), &vault);
                vault.borrow_mut().preload();
                let snap = vault.borrow().snapshot();
                let _ = page.borrow().evaluate_script(&js_set_blobs(&snap));
            }
            Event::UserEvent(Msg::Boost) => {
                let on = !desk.borrow().prefs.boost;
                desk.borrow_mut().set_boost(on);
                let urls = if on {
                    boost_targets(&desk.borrow())
                } else {
                    Vec::new()
                };
                let hosts = boost_hosts(&urls);
                let bjs = js_set_boost(on);
                let _ = toolbar.borrow().evaluate_script(&bjs);
                let url = tabs.borrow().current().url().to_string();
                if host::is_chrome_url(&url) {
                    let _ = page.borrow().evaluate_script(&bjs);
                }
                if on {
                    snap_desk(&desk.borrow(), &vault);
                    vault.borrow_mut().preload();
                    if host::is_chrome_url(&url) {
                        let _ = page.borrow().evaluate_script(&js_boost_warm(&urls));
                    }
                } else {
                    let _ = page.borrow().evaluate_script(
                        "window.__vapurrBoostClear && window.__vapurrBoostClear()",
                    );
                }
                #[cfg(windows)]
                {
                    let lvl = MemoryUsageLevel::Normal;
                    let _ = sidebar.borrow().set_memory_usage_level(lvl);
                    let _ = toolbar.borrow().set_memory_usage_level(lvl);
                    let _ = page.borrow().set_memory_usage_level(lvl);
                }
                let snap = vault.borrow().snapshot();
                let js = js_on_boost(on, &snap, &hosts);
                let _ = toolbar.borrow().evaluate_script(&js);
                if host::is_chrome_url(&url) {
                    let _ = page.borrow().evaluate_script(&js);
                    if url.contains("memory.html") {
                        let _ = page.borrow().evaluate_script(&js_set_blobs(&snap));
                    }
                }
                paint_chrome();
            }
            Event::UserEvent(Msg::Wallet) => {
                let _ = wallet_tx.send(vapurr_wallet::WalletCmd::Snap);
            }
            Event::UserEvent(Msg::WalletSend { asset, to, amt }) => {
                let _ = wallet_tx.send(vapurr_wallet::WalletCmd::Send { asset, to, amt });
            }
            Event::UserEvent(Msg::WalletExec {
                to,
                data,
                value,
                chain_id,
                gas,
            }) => {
                let _ = wallet_tx.send(vapurr_wallet::WalletCmd::Exec {
                    to,
                    data,
                    value,
                    chain_id,
                    gas,
                });
            }
            Event::UserEvent(Msg::WalletImport { secret }) => {
                let _ = wallet_tx.send(vapurr_wallet::WalletCmd::Import { secret });
            }
            Event::UserEvent(Msg::WalletSetNet(net)) => {
                let _ = wallet_tx.send(vapurr_wallet::WalletCmd::SetNet(net));
            }
            Event::UserEvent(Msg::WalletRevealSeed) => {
                let _ = wallet_tx.send(vapurr_wallet::WalletCmd::RevealSeed);
            }
            Event::UserEvent(Msg::WalletExportKey) => {
                let _ = wallet_tx.send(vapurr_wallet::WalletCmd::ExportKey);
            }
            Event::UserEvent(Msg::WalletResolve { to }) => {
                let _ = wallet_tx.send(vapurr_wallet::WalletCmd::Resolve { to });
            }
            Event::UserEvent(Msg::LoginStatus) => {
                let _ = wallet_tx.send(vapurr_wallet::WalletCmd::LoginStatus);
            }
            Event::UserEvent(Msg::LoginContinue) => {
                let _ = wallet_tx.send(vapurr_wallet::WalletCmd::LoginContinue);
            }
            Event::UserEvent(Msg::LoginCreate) => {
                let _ = wallet_tx.send(vapurr_wallet::WalletCmd::LoginCreate);
            }
            Event::UserEvent(Msg::LoginRestore { secret }) => {
                let _ = wallet_tx.send(vapurr_wallet::WalletCmd::LoginRestore { secret });
            }
            Event::UserEvent(Msg::PatchApply) => match patch::apply_and_relaunch() {
                Ok(()) => {
                    crash::log("patch apply staged; exiting for swap");
                    *control_flow = ControlFlow::Exit;
                }
                Err(e) => {
                    crash::log(&format!("patch apply: {e}"));
                    let js = format!(
                        "window.__onPatch && window.__onPatch({})",
                        serde_json::json!({ "ok": false, "error": e })
                    );
                    let _ = page.borrow().evaluate_script(&js);
                }
            },
            Event::UserEvent(Msg::Logout) => {
                let _ = wallet_tx.send(vapurr_wallet::WalletCmd::Logout);
            }
            Event::UserEvent(Msg::WalletSnap(snap)) => {
                *last_wallet.borrow_mut() = snap.clone();
                if let Some(a) = snap.get("address").and_then(|x| x.as_str()) {
                    host::adopt_wallet_address(a);
                }
                let _ = page.borrow().evaluate_script(&js_set_wallet(&snap));
            }
            Event::UserEvent(Msg::WalletErr(msg)) => {
                let js = format!(
                    "window.__walletErr && window.__walletErr({})",
                    serde_json::to_string(&msg).unwrap_or_else(|_| "\"failed\"".into())
                );
                let _ = page.borrow().evaluate_script(&js);
            }
            Event::UserEvent(Msg::ZzzmailSend { to, body, asset }) => {
                let snap = host::zzzmail_send_json(&to, &body, &asset);
                let _ = page.borrow().evaluate_script(&js_set_mail(&snap));
            }
            Event::UserEvent(Msg::ZzzmailInbox) => {
                let snap = host::zzzmail_inbox_json();
                let _ = page.borrow().evaluate_script(&js_set_mail(&snap));
            }
            Event::UserEvent(Msg::ZzzmailHood { name }) => {
                let snap = host::zzzmail_hood_register_json(&name);
                let _ = page.borrow().evaluate_script(&js_set_mail(&snap));
            }
            Event::UserEvent(Msg::Econ) => {
                let _ = econ_tx.send(vapurr_econ::EconCmd::Snap);
            }
            Event::UserEvent(Msg::EconMint(amt)) => {
                let _ = econ_tx.send(vapurr_econ::EconCmd::Mint(amt));
            }
            Event::UserEvent(Msg::EconRedeem(amt)) => {
                let _ = econ_tx.send(vapurr_econ::EconCmd::Redeem(amt));
            }
            Event::UserEvent(Msg::EconDeploy) => {
                let _ = econ_tx.send(vapurr_econ::EconCmd::Deploy);
            }
            Event::UserEvent(Msg::EconSeed { usdg, vapurr }) => {
                let _ = econ_tx.send(vapurr_econ::EconCmd::Seed { usdg, vapurr });
            }
            Event::UserEvent(Msg::EconSnap(snap)) => {
                *last_econ.borrow_mut() = snap.clone();
                let _ = page.borrow().evaluate_script(&js_set_econ(&snap));
            }
            Event::UserEvent(Msg::EconErr { which, msg }) => {
                let _ = page.borrow().evaluate_script(&js_econ_err(&which, &msg));
            }
            Event::UserEvent(Msg::Outbid) => {
                let _ = econ_tx.send(vapurr_econ::EconCmd::Outbid);
            }
            Event::UserEvent(Msg::OutbidBid { url, title, amt }) => {
                let _ = econ_tx.send(vapurr_econ::EconCmd::OutbidBid { url, title, amt });
            }
            Event::UserEvent(Msg::OutbidDeploy) => {
                let _ = econ_tx.send(vapurr_econ::EconCmd::OutbidDeploy);
            }
            Event::UserEvent(Msg::KetList) => {
                let _ = econ_tx.send(vapurr_econ::EconCmd::KetList);
            }
            Event::UserEvent(Msg::KetListPay {
                token,
                pool,
                symbol,
                name,
                amt,
                meta,
            }) => {
                let _ = econ_tx.send(vapurr_econ::EconCmd::KetListPay {
                    token,
                    pool,
                    symbol,
                    name,
                    amt,
                    meta,
                });
            }
            Event::UserEvent(Msg::KetListDeploy) => {
                let _ = econ_tx.send(vapurr_econ::EconCmd::KetListDeploy);
            }
            Event::UserEvent(Msg::LoopDeploy) => {
                let _ = econ_tx.send(vapurr_econ::EconCmd::LoopDeploy);
            }
            Event::UserEvent(Msg::LoopOp { op, amt, steps }) => {
                let _ = econ_tx.send(vapurr_econ::EconCmd::LoopOp { op, amt, steps });
            }
            Event::UserEvent(Msg::HouseDeploy) => {
                let _ = econ_tx.send(vapurr_econ::EconCmd::HouseDeploy);
            }
            Event::UserEvent(Msg::HouseSeed { vapurr, pusd }) => {
                let _ = econ_tx.send(vapurr_econ::EconCmd::HouseSeed { vapurr, pusd });
            }
            Event::UserEvent(Msg::HouseBootstrap) => {
                let _ = econ_tx.send(vapurr_econ::EconCmd::HouseBootstrap);
            }
            Event::UserEvent(Msg::HouseSwap { sell_v, amt }) => {
                let _ = econ_tx.send(vapurr_econ::EconCmd::HouseSwap { sell_v, amt });
            }
            Event::UserEvent(Msg::OutbidSnap(snap)) => {
                *last_outbid.borrow_mut() = snap.clone();
                let _ = page.borrow().evaluate_script(&js_set_outbid(&snap));
            }
            Event::UserEvent(Msg::KetListSnap(snap)) => {
                *last_ketlist.borrow_mut() = snap.clone();
                let _ = page.borrow().evaluate_script(&js_set_ketlist(&snap));
            }
            Event::UserEvent(Msg::Desk) => {
                let djson = desk_json(&desk.borrow(), &shield);
                let _ = page.borrow().evaluate_script(&js_set_desk(&djson));
                let url = tabs.borrow().current().url().to_string();
                if url.contains("cookies.html") {
                    cookies::push(&page.borrow(), &url);
                }
            }
            Event::UserEvent(Msg::PageStart(url)) => {
                set_page_url(&page_url, &url);
                inject_faucet(&page.borrow(), &url);
                inject_shield(&page.borrow(), &shield, &url);
                if wants_wallet_snap(&url) {
                    let wj = last_wallet.borrow().clone();
                    if wj.is_object() && !wj.as_object().map(|o| o.is_empty()).unwrap_or(true) {
                        let _ = page.borrow().evaluate_script(&js_set_wallet(&wj));
                    }
                    let _ = wallet_tx.send(vapurr_wallet::WalletCmd::Snap);
                }
                if url.contains("ketcharts.html") {
                    let kj = last_ketlist.borrow().clone();
                    if kj.is_object() && !kj.as_object().map(|o| o.is_empty()).unwrap_or(true) {
                        let _ = page.borrow().evaluate_script(&js_set_ketlist(&kj));
                    }
                    let _ = econ_tx.send(vapurr_econ::EconCmd::KetList);
                }
                if url.contains("login.html") {
                    let _ = wallet_tx.send(vapurr_wallet::WalletCmd::LoginStatus);
                }
            }
            Event::UserEvent(Msg::ShieldDom { ids, classes }) => {
                let url = page_url.lock().map(|s| s.clone()).unwrap_or_default();
                let js = shield.extra_inject_js(&url, &classes, &ids);
                if !js.is_empty() {
                    let _ = page.borrow().evaluate_script(&js);
                }
            }
            Event::UserEvent(Msg::Title(title)) => {
                tabs.borrow_mut().set_title(title.clone());
                let url = tabs.borrow().current().url().to_string();
                let _ = desk.borrow_mut().touch_title(&url, &title);
                paint_chrome();
            }
            Event::UserEvent(Msg::Url(url)) => {
                let suppress = tabs.borrow().suppress;
                if !suppress {
                    tabs.borrow_mut().observe(url.clone());
                }
                tabs.borrow_mut().suppress = false;
                set_page_url(&page_url, &url);
                let _ = toolbar.borrow().evaluate_script(&js_set_url(&url));
                let title = tabs.borrow().current().title.clone();
                desk.borrow_mut().record_nav(&url, &title);
                inject_faucet(&page.borrow(), &url);
                inject_shield(&page.borrow(), &shield, &url);
                if url.contains("vapurr.localhost") {
                    let snap = last_chain.borrow().clone();
                    if !snap.is_empty() {
                        let _ = page.borrow().evaluate_script(&js_set_chain(&snap));
                        let _ = toolbar.borrow().evaluate_script(&js_set_chain(&snap));
                    }
                    let djson = desk_json(&desk.borrow(), &shield);
                    let _ = page.borrow().evaluate_script(&js_set_desk(&djson));
                    if url.contains("memory.html") {
                        let snap = vault.borrow().snapshot();
                        let _ = page.borrow().evaluate_script(&js_set_blobs(&snap));
                    }
                    if url.contains("zzzmail.html") || url.contains("zmail.html") {
                        let snap = host::zzzmail_inbox_json();
                        let _ = page.borrow().evaluate_script(&js_set_mail(&snap));
                    }
                    if wants_wallet_snap(&url) {
                        let wj = last_wallet.borrow().clone();
                        if wj.is_object() && !wj.as_object().map(|o| o.is_empty()).unwrap_or(true) {
                            let _ = page.borrow().evaluate_script(&js_set_wallet(&wj));
                        }
                        let _ = wallet_tx.send(vapurr_wallet::WalletCmd::Snap);
                    }
                    if url.contains("login.html") {
                        let _ = wallet_tx.send(vapurr_wallet::WalletCmd::LoginStatus);
                    }
                    if url.contains("cookies.html") {
                        cookies::push(&page.borrow(), &url);
                    }
                }
                if desk.borrow().prefs.boost && host::is_chrome_url(&url) {
                    let urls = boost_targets(&desk.borrow());
                    let _ = page.borrow().evaluate_script(&js_boost_warm(&urls));
                }
                paint_chrome();
            }
            Event::UserEvent(Msg::Chain(json)) => {
                *last_chain.borrow_mut() = json.clone();
                let js = js_set_chain(&json);
                let url = tabs.borrow().current().url().to_string();
                if host::is_chrome_url(&url) {
                    let _ = page.borrow().evaluate_script(&js);
                }
                let _ = toolbar.borrow().evaluate_script(&js);
            }
            Event::WindowEvent {
                event: WindowEvent::Focused(on),
                ..
            } => {
                host::set_focused(on);
                #[cfg(windows)]
                {
                    let boost = desk.borrow().prefs.boost;
                    let lvl = if on || boost {
                        MemoryUsageLevel::Normal
                    } else {
                        MemoryUsageLevel::Low
                    };
                    let _ = sidebar.borrow().set_memory_usage_level(lvl);
                    let _ = toolbar.borrow().set_memory_usage_level(lvl);
                    let _ = page.borrow().set_memory_usage_level(lvl);
                    let _ = radio.borrow().set_memory_usage_level(lvl);
                }
            }
            Event::UserEvent(Msg::RadioLayout {
                float,
                corner,
                collapsed,
            }) => {
                {
                    let mut ui = radio_ui.borrow_mut();
                    ui.float = float;
                    ui.collapsed = collapsed;
                    ui.corner = parse_radio_corner(&corner);
                }
                let log: LogicalSize<f64> = window.inner_size().to_logical(window.scale_factor());
                let (s, t, c, r) = layout(log.width, log.height, &radio_ui.borrow());
                let _ = sidebar.borrow().set_bounds(s);
                let _ = toolbar.borrow().set_bounds(t);
                let _ = page.borrow().set_bounds(c);
                let _ = radio.borrow().set_bounds(r);
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                let log: LogicalSize<f64> = size.to_logical(window.scale_factor());
                let (s, t, c, r) = layout(log.width, log.height, &radio_ui.borrow());
                let _ = sidebar.borrow().set_bounds(s);
                let _ = toolbar.borrow().set_bounds(t);
                let _ = page.borrow().set_bounds(c);
                let _ = radio.borrow().set_bounds(r);
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                crash::log("CloseRequested");
                snap_desk(&desk.borrow(), &vault);
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{pane_url, vapurr_url, FOMO_FAMILY};

    #[test]
    fn wallet_is_chrome_fomo_is_the_site() {
        let w = pane_url("wallet");
        assert!(
            w.contains("wallet.html") || w.contains("login.html?next=wallet"),
            "{w}"
        );
        let p = pane_url("portfolio");
        assert!(
            p.contains("wallet.html") || p.contains("login.html?next=portfolio"),
            "{p}"
        );
        assert_eq!(pane_url("fomo"), FOMO_FAMILY);
        assert_eq!(pane_url("family"), FOMO_FAMILY);
        assert_eq!(pane_url("id"), vapurr_url("id.html"));
        assert_eq!(pane_url("shield"), vapurr_url("shield.html"));
        assert_eq!(pane_url("adblock"), vapurr_url("shield.html"));
    }

    #[test]
    fn ketbook_is_chrome() {
        assert_eq!(pane_url("ketbook"), vapurr_url("ketbook.html"));
        assert_eq!(pane_url("docs"), vapurr_url("ketbook.html"));
        assert_eq!(pane_url("honkit"), vapurr_url("ketbook.html"));
        assert_eq!(pane_url("lithe"), vapurr_url("pusd.html"));
        assert_eq!(pane_url("pusd"), vapurr_url("pusd.html"));
        assert_eq!(pane_url("euler"), vapurr_url("pusd.html?tab=euler"));
        assert_eq!(pane_url("loop"), vapurr_url("pusd.html?tab=euler"));
        assert_eq!(pane_url("ketpay"), vapurr_url("pay.html"));
        assert_eq!(pane_url("pay"), vapurr_url("pay.html"));
        assert_ne!(pane_url("404"), vapurr_url("pay.html"));
    }
}
