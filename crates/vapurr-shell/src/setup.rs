//! Branded first-run. Same chrome as the app. No admin.

use std::borrow::Cow;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use wry::http::{header::CONTENT_TYPE, Response};

use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::{Theme, WindowBuilder};
use wry::{WebContext, WebViewBuilder};

#[cfg(windows)]
use wry::WebViewBuilderExtWindows;

#[cfg(windows)]
use tao::platform::windows::{EventLoopBuilderExtWindows, WindowBuilderExtWindows};

use crate::crash;
use crate::host;
use crate::{app_icon, fatal, paint_caption};

const SETUP_VER: &str = env!("CARGO_PKG_VERSION");
const AUMID_SETUP: &str = "vapurr.Setup";

const SETUP_BOOT_JS: &str = r#"
document.addEventListener("contextmenu", function (e) { e.preventDefault(); });
document.addEventListener("dragstart", function (e) { e.preventDefault(); });
document.documentElement.setAttribute("data-theme", "dark");
window.__vapurrPost = function (msg) {
  try {
    var p = JSON.stringify(msg);
    if (window.ipc && window.ipc.postMessage) window.ipc.postMessage(p);
    else if (window.chrome && window.chrome.webview) window.chrome.webview.postMessage(p);
  } catch (err) {}
};
"#;

#[derive(Clone)]
enum Msg {
    Info,
    Install { desktop: bool },
    InstallFailed(String),
    Launch,
    Portable,
    Quit,
}

pub fn wants_setup(args: &[String]) -> bool {
    if args.iter().any(|a| a == "--app" || a == "--uninstall") {
        return false;
    }
    if args.iter().any(|a| a == "--setup") {
        return true;
    }
    is_setup_name(&exe_stem())
}

pub fn is_setup_name(stem: &str) -> bool {
    let s = stem.trim().to_ascii_lowercase();
    s == "vapurr-setup" || s == "install vapurr" || s.starts_with("install ")
}

pub fn install_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("USERPROFILE").map(|h| PathBuf::from(h).join("AppData/Local"))
        })
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("Programs").join("vapurr")
}

fn exe_stem() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_default()
}

fn current_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn installed_exe() -> PathBuf {
    install_dir().join("vapurr.exe")
}

fn already_installed() -> bool {
    installed_exe().is_file()
}

fn start_menu_lnk() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("Microsoft/Windows/Start Menu/Programs/vapurr.lnk")
}

fn desktop_lnk() -> PathBuf {
    let base = std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("Desktop/vapurr.lnk")
}

fn wide(s: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
    s.as_ref().encode_wide().chain(Some(0)).collect()
}

pub fn set_aumid(id: &str) {
    #[cfg(windows)]
    {
        use windows::core::HSTRING;
        use windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;
        let s = HSTRING::from(id);
        let _ = unsafe { SetCurrentProcessExplicitAppUserModelID(&s) };
    }
    #[cfg(not(windows))]
    {
        let _ = id;
    }
}

fn write_lnk(path: &Path, target: &Path) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    #[cfg(windows)]
    {
        use windows::core::{Interface, PCWSTR};
        use windows::Win32::Foundation::TRUE;
        use windows::Win32::System::Com::{
            CoCreateInstance, CoInitializeEx, IPersistFile, CLSCTX_INPROC_SERVER,
            COINIT_APARTMENTTHREADED,
        };
        use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
        use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
        let wd = target.parent().unwrap_or(Path::new("."));
        let path_w = wide(path);
        let target_w = wide(target);
        let wd_w = wide(wd);
        let desc_w = wide("vapurr");
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let sl: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
                .map_err(|e| e.to_string())?;
            sl.SetPath(PCWSTR(target_w.as_ptr()))
                .map_err(|e| e.to_string())?;
            sl.SetWorkingDirectory(PCWSTR(wd_w.as_ptr()))
                .map_err(|e| e.to_string())?;
            sl.SetIconLocation(PCWSTR(target_w.as_ptr()), 0)
                .map_err(|e| e.to_string())?;
            sl.SetDescription(PCWSTR(desc_w.as_ptr()))
                .map_err(|e| e.to_string())?;
            let _ = sl.SetShowCmd(SW_SHOWNORMAL);
            let pf: IPersistFile = sl.cast().map_err(|e| e.to_string())?;
            pf.Save(PCWSTR(path_w.as_ptr()), TRUE)
                .map_err(|e| e.to_string())?;
        }
        if !path.is_file() {
            return Err("shortcut missing".into());
        }
        return Ok(());
    }
    #[cfg(not(windows))]
    {
        let _ = (path, target);
        Err("shortcuts are Windows-only".into())
    }
}

fn reg_set_sz(
    key: windows::Win32::System::Registry::HKEY,
    name: windows::core::PCWSTR,
    val: &str,
) {
    use windows::Win32::System::Registry::{RegSetValueExW, REG_SZ};
    let data = wide(val);
    let bytes = unsafe {
        std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 2)
    };
    let _ = unsafe { RegSetValueExW(key, name, 0, REG_SZ, Some(bytes)) };
}

fn reg_set_dword(
    key: windows::Win32::System::Registry::HKEY,
    name: windows::core::PCWSTR,
    val: u32,
) {
    use windows::Win32::System::Registry::{RegSetValueExW, REG_DWORD};
    let _ = unsafe { RegSetValueExW(key, name, 0, REG_DWORD, Some(&val.to_le_bytes())) };
}

fn write_uninstall_key(exe: &Path, dir: &Path) {
    use windows::core::w;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, HKEY_CURRENT_USER, KEY_WRITE, REG_OPTION_NON_VOLATILE,
    };
    use windows::Win32::System::SystemInformation::GetLocalTime;
    let mut hkey = windows::Win32::System::Registry::HKEY::default();
    let err = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\vapurr"),
            0,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut hkey,
            None,
        )
    };
    if err.is_err() {
        return;
    }
    let uninst = format!("\"{}\" --uninstall", exe.display());
    let icon = format!("{},0", exe.display());
    let size_kb = (std::fs::metadata(exe).map(|m| m.len()).unwrap_or(21_000_000) / 1024) as u32
        + 180;
    let st = unsafe { GetLocalTime() };
    let date = format!("{:04}{:02}{:02}", st.wYear, st.wMonth, st.wDay);
    reg_set_sz(hkey, w!("DisplayName"), "vapurr");
    reg_set_sz(hkey, w!("DisplayVersion"), SETUP_VER);
    reg_set_sz(hkey, w!("Publisher"), "vapurr");
    reg_set_sz(hkey, w!("DisplayIcon"), &icon);
    reg_set_sz(hkey, w!("InstallLocation"), &dir.display().to_string());
    reg_set_sz(hkey, w!("UninstallString"), &uninst);
    reg_set_sz(hkey, w!("QuietUninstallString"), &uninst);
    reg_set_sz(hkey, w!("InstallDate"), &date);
    reg_set_dword(hkey, w!("NoModify"), 1);
    reg_set_dword(hkey, w!("NoRepair"), 1);
    reg_set_dword(hkey, w!("EstimatedSize"), size_kb);
    unsafe {
        let _ = RegCloseKey(hkey);
    }
}

fn copy_beside(name: &str, dest_dir: &Path) {
    let src = current_dir().join(name);
    if src.is_file() {
        let _ = std::fs::copy(&src, dest_dir.join(name));
    }
}

fn do_install(desktop: bool) -> Result<PathBuf, String> {
    let dest = install_dir();
    crash::log(&format!("install → {}", dest.display()));
    std::fs::create_dir_all(&dest).map_err(|e| format!("create dir: {e}"))?;
    let me = std::env::current_exe().map_err(|e| e.to_string())?;
    let target = dest.join("vapurr.exe");
    if me.canonicalize().ok() != target.canonicalize().ok() {
        std::fs::copy(&me, &target).map_err(|e| format!("copy exe: {e}"))?;
        crash::log("copied exe");
    }
    let loader = current_dir().join("WebView2Loader.dll");
    if loader.is_file() {
        std::fs::copy(&loader, dest.join("WebView2Loader.dll"))
            .map_err(|e| format!("copy loader: {e}"))?;
    } else {
        crash::log("no WebView2Loader.dll beside installer — using system runtime");
    }
    copy_beside("VERSION.txt", &dest);
    copy_beside("LICENSE.txt", &dest);
    let _ = std::fs::remove_file(start_menu_lnk().with_extension("url"));
    let _ = std::fs::remove_file(desktop_lnk().with_extension("url"));
    if let Err(e) = write_lnk(&start_menu_lnk(), &target) {
        crash::log(&format!("start menu: {e}"));
    }
    if desktop {
        if let Err(e) = write_lnk(&desktop_lnk(), &target) {
            crash::log(&format!("desktop: {e}"));
        }
    }
    write_uninstall_key(&target, &dest);
    crash::log(&format!("installed {}", target.display()));
    Ok(target)
}

fn spawn_detached(exe: &Path) -> Result<(), String> {
    crash::log(&format!("launch {}", exe.display()));
    let cmd = format!("\"{}\"", exe.display());
    spawn_process(exe, &cmd, exe.parent().unwrap_or(Path::new(".")), false)
}

fn spawn_process(exe: &Path, cmdline: &str, dir: &Path, hidden: bool) -> Result<(), String> {
    use windows::core::{PCWSTR, PWSTR};
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::Threading::{
        CreateProcessW, CREATE_BREAKAWAY_FROM_JOB, CREATE_NEW_PROCESS_GROUP,
        CREATE_NO_WINDOW, CREATE_UNICODE_ENVIRONMENT, DETACHED_PROCESS, PROCESS_INFORMATION,
        STARTF_USESHOWWINDOW, STARTUPINFOW,
    };
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let exe_w = wide(exe);
    let mut cmd_w = wide(cmdline);
    let dir_w = wide(dir);
    let mut si = STARTUPINFOW::default();
    si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
    si.dwFlags = STARTF_USESHOWWINDOW;
    si.wShowWindow = if hidden { 0 } else { SW_SHOWNORMAL.0 as u16 };
    let mut pi = PROCESS_INFORMATION::default();
    let mut flags = CREATE_BREAKAWAY_FROM_JOB | CREATE_NEW_PROCESS_GROUP | CREATE_UNICODE_ENVIRONMENT;
    if hidden {
        flags |= CREATE_NO_WINDOW | DETACHED_PROCESS;
    }
    unsafe {
        CreateProcessW(
            PCWSTR(exe_w.as_ptr()),
            PWSTR(cmd_w.as_mut_ptr()),
            None,
            None,
            false,
            flags,
            None,
            PCWSTR(dir_w.as_ptr()),
            &si,
            &mut pi,
        )
        .map_err(|e| e.to_string())?;
        let _ = CloseHandle(HANDLE(pi.hThread.0));
        let _ = CloseHandle(HANDLE(pi.hProcess.0));
    }
    Ok(())
}

pub fn install_now(desktop: bool) -> Result<PathBuf, String> {
    let exe = do_install(desktop)?;
    spawn_detached(&exe)?;
    Ok(exe)
}

pub fn uninstall_silent() {
    crash::log("uninstall");
    let dest = install_dir();
    let _ = std::fs::remove_file(start_menu_lnk());
    let _ = std::fs::remove_file(desktop_lnk());
    let _ = std::fs::remove_file(start_menu_lnk().with_extension("url"));
    let _ = std::fs::remove_file(desktop_lnk().with_extension("url"));
    use windows::core::w;
    use windows::Win32::System::Registry::{RegDeleteTreeW, HKEY_CURRENT_USER};
    let _ = unsafe {
        RegDeleteTreeW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\vapurr"),
        )
    };
    let dir = dest.display().to_string();
    let sys = std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
    let cmd = sys.join("System32").join("cmd.exe");
    let cmdline = format!(
        "\"{}\" /C timeout /T 2 /NOBREAK >nul & rmdir /S /Q \"{dir}\"",
        cmd.display()
    );
    let _ = spawn_process(&cmd, &cmdline, Path::new("."), true);
}

fn parse(body: &str) -> Option<Msg> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    match v.get("cmd")?.as_str()? {
        "setup-info" => Some(Msg::Info),
        "setup-install" => Some(Msg::Install {
            desktop: v.get("desktop").and_then(|x| x.as_bool()).unwrap_or(true),
        }),
        "setup-launch" => Some(Msg::Launch),
        "setup-portable" => Some(Msg::Portable),
        "setup-quit" => Some(Msg::Quit),
        _ => None,
    }
}

fn info_json() -> String {
    serde_json::json!({
        "installed": already_installed(),
        "dest": install_dir().display().to_string(),
        "version": SETUP_VER,
    })
    .to_string()
}

fn json_resp(v: serde_json::Value) -> Response<Cow<'static, [u8]>> {
    Response::builder()
        .header(CONTENT_TYPE, "application/json; charset=utf-8")
        .header("Cache-Control", "no-store")
        .header("Access-Control-Allow-Origin", "*")
        .body(Cow::Owned(v.to_string().into_bytes()))
        .unwrap()
}

fn setup_api_kind(path: &str) -> Option<&str> {
    path.trim_start_matches('/')
        .split('?')
        .next()
        .unwrap_or("")
        .strip_prefix("setup/api/")
        .map(|s| s.trim_end_matches('/'))
}

fn install_desktop(kind: &str, body: &[u8]) -> bool {
    let rest = kind
        .strip_prefix("install")
        .unwrap_or(kind)
        .trim_matches('/');
    if rest == "0" || rest.eq_ignore_ascii_case("nodesk") {
        return false;
    }
    if rest == "1" || rest.eq_ignore_ascii_case("desk") {
        return true;
    }
    serde_json::from_slice::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v.get("desktop").and_then(|x| x.as_bool()))
        .unwrap_or(true)
}

fn setup_http(
    req: &wry::http::Request<Vec<u8>>,
    proxy: &tao::event_loop::EventLoopProxy<Msg>,
) -> Option<Response<Cow<'static, [u8]>>> {
    let kind = setup_api_kind(req.uri().path())?;
    crash::log(&format!("setup http {kind}"));
    if kind == "info" {
        return Some(json_resp(
            serde_json::from_str(&info_json()).unwrap_or(serde_json::json!({})),
        ));
    }
    if kind == "install" || kind.starts_with("install/") {
        let _ = proxy.send_event(Msg::Install {
            desktop: install_desktop(kind, req.body()),
        });
        return Some(json_resp(serde_json::json!({ "ok": true })));
    }
    if kind == "launch" {
        let _ = proxy.send_event(Msg::Launch);
        return Some(json_resp(serde_json::json!({ "ok": true })));
    }
    if kind == "portable" {
        let _ = proxy.send_event(Msg::Portable);
        return Some(json_resp(serde_json::json!({ "ok": true })));
    }
    if kind == "quit" {
        let _ = proxy.send_event(Msg::Quit);
        return Some(json_resp(serde_json::json!({ "ok": true })));
    }
    Some(json_resp(serde_json::json!({ "ok": false, "error": "unknown" })))
}

fn center_window(window: &tao::window::Window) {
    use tao::dpi::PhysicalPosition;
    let Some(m) = window.current_monitor() else {
        return;
    };
    let ms = m.size();
    let mp = m.position();
    let ws = window.outer_size();
    let x = mp.x + (ms.width as i32 - ws.width as i32) / 2;
    let y = mp.y + (ms.height as i32 - ws.height as i32) / 2;
    window.set_outer_position(PhysicalPosition::new(x, y));
}

pub fn run() {
    crash::log("setup");
    set_aumid(AUMID_SETUP);
    let mut event_loop_b = EventLoopBuilder::<Msg>::with_user_event();
    #[cfg(windows)]
    {
        event_loop_b.with_theme(Some(Theme::Dark));
    }
    let event_loop = event_loop_b.build();
    let proxy = event_loop.create_proxy();
    let mut window_b = WindowBuilder::new()
        .with_title("Install vapurr")
        .with_theme(Some(Theme::Dark))
        .with_inner_size(LogicalSize::new(440.0, 580.0))
        .with_resizable(false)
        .with_visible(false)
        .with_window_icon(app_icon(32));
    #[cfg(windows)]
    {
        window_b = window_b
            .with_taskbar_icon(app_icon(256))
            .with_window_classname("VAPURR-SETUP");
    }
    let window = window_b
        .build(&event_loop)
        .unwrap_or_else(|e| fatal("window", e));
    #[cfg(windows)]
    paint_caption(&window, false);
    center_window(&window);

    let mut web_ctx = WebContext::new(Some(host::wv2_data_dir()));
    let ipc = {
        let proxy = proxy.clone();
        move |req: wry::http::Request<String>| {
            if let Some(msg) = parse(req.body()) {
                let _ = proxy.send_event(msg);
            }
        }
    };
    let mut page_b = WebViewBuilder::with_web_context(&mut web_ctx)
        .with_background_color((0x0E, 0x0E, 0x0E, 255))
        .with_custom_protocol("vapurr".into(), {
            let proxy = proxy.clone();
            move |id, req| {
                if let Some(resp) = setup_http(&req, &proxy) {
                    return resp;
                }
                host::serve(id, req)
            }
        })
        .with_initialization_script(SETUP_BOOT_JS)
        .with_ipc_handler(ipc)
        .with_devtools(cfg!(debug_assertions))
        .with_url(crate::nav::vapurr_url("setup.html"));
    #[cfg(windows)]
    {
        page_b = page_b.with_additional_browser_args(host::WV_ARGS);
    }
    let page = Rc::new(
        page_b
            .build(&window)
            .unwrap_or_else(|e| fatal("page", e)),
    );
    window.set_visible(true);

    let mut started = false;
    let loop_proxy = proxy.clone();
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::UserEvent(Msg::Info) => {
                let js = format!(
                    "window.__setup && window.__setup({})",
                    info_json()
                );
                let _ = page.evaluate_script(&js);
            }
            Event::UserEvent(Msg::Install { desktop }) => {
                if started {
                    return;
                }
                started = true;
                let proxy = loop_proxy.clone();
                let _ = std::thread::Builder::new()
                    .name("setup-install".into())
                    .spawn(move || match do_install(desktop) {
                        Ok(exe) => {
                            if spawn_detached(&exe).is_ok() {
                                let _ = proxy.send_event(Msg::Quit);
                            } else {
                                let _ = proxy.send_event(Msg::InstallFailed("could not open".into()));
                            }
                        }
                        Err(e) => {
                            let _ = proxy.send_event(Msg::InstallFailed(e));
                        }
                    });
            }
            Event::UserEvent(Msg::InstallFailed(e)) => {
                started = false;
                crash::log(&format!("install failed: {e}"));
                let js = format!(
                    "(function(){{var e=document.getElementById('err');var g=document.getElementById('go');if(e)e.textContent={};if(g){{g.disabled=false;g.textContent='Install on this PC';}}}})()",
                    serde_json::to_string(&e).unwrap_or_else(|_| "\"install failed\"".into())
                );
                let _ = page.evaluate_script(&js);
            }
            Event::UserEvent(Msg::Launch) => {
                if started {
                    return;
                }
                let exe = installed_exe();
                if exe.is_file() {
                    started = true;
                    let _ = spawn_detached(&exe);
                    *control_flow = ControlFlow::Exit;
                }
            }
            Event::UserEvent(Msg::Quit) => {
                *control_flow = ControlFlow::Exit;
            }
            Event::UserEvent(Msg::Portable) => {
                if started {
                    return;
                }
                started = true;
                let portable = current_dir().join("vapurr.exe");
                if portable.is_file() {
                    let _ = spawn_detached(&portable);
                } else if let Ok(me) = std::env::current_exe() {
                    let cmd = format!("\"{}\" --app", me.display());
                    let _ = spawn_process(&me, &cmd, &current_dir(), false);
                }
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_names() {
        assert!(is_setup_name("Install vapurr"));
        assert!(is_setup_name("vapurr-setup"));
        assert!(is_setup_name("install vapurr"));
        assert!(!is_setup_name("vapurr"));
    }

    #[test]
    fn setup_api_install_slash() {
        assert_eq!(setup_api_kind("/setup/api/install/1"), Some("install/1"));
        assert!(install_desktop("install/1", b"{}"));
        assert!(!install_desktop("install/0", b"{}"));
    }

    #[test]
    fn install_dir_is_per_user() {
        let d = install_dir();
        let s = d.to_string_lossy();
        assert!(s.contains("Programs"), "{s}");
        assert!(s.ends_with("vapurr"), "{s}");
        assert!(!s.contains("crates"));
    }

    #[test]
    fn install_is_a_get_route() {
        assert_eq!(setup_api_kind("/setup/api/info"), Some("info"));
        assert_eq!(setup_api_kind("/setup/api/install/1"), Some("install/1"));
        assert_eq!(setup_api_kind("/setup/api/install/0"), Some("install/0"));
        assert_eq!(setup_api_kind("setup/api/portable"), Some("portable"));
        assert!(setup_api_kind("/mascot.png").is_none());
        assert!(install_desktop("install/1", b""));
        assert!(!install_desktop("install/0", b""));
        assert!(install_desktop("install", b""));
        assert!(!install_desktop("install", br#"{"desktop":false}"#));
    }
}
