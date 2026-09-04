//! Wire Brave adblock-rust into WebView2 WebResourceRequested.

use std::sync::Arc;

use vapurr_shield::{resource_type_from_webview, Shield};

#[cfg(windows)]
pub fn attach(page: &wry::WebView, shield: Arc<Shield>, page_url: Arc<std::sync::Mutex<String>>) {
    use webview2_com::Microsoft::Web::WebView2::Win32::*;
    use webview2_com::{take_pwstr, WebResourceRequestedEventHandler};
    use windows::core::{Interface, HSTRING, PWSTR};
    use wry::WebViewExtWindows;

    let controller = page.controller();
    unsafe {
        let Ok(webview) = controller.CoreWebView2() else {
            tracing::warn!("shield: no CoreWebView2");
            return;
        };
        let env = match webview
            .cast::<ICoreWebView2_2>()
            .and_then(|v| v.Environment())
        {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("shield: no Environment: {e}");
                return;
            }
        };
        let filter = HSTRING::from("*");
        let filtered = if let Ok(v22) = webview.cast::<ICoreWebView2_22>() {
            v22.AddWebResourceRequestedFilterWithRequestSourceKinds(
                &filter,
                COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL,
                COREWEBVIEW2_WEB_RESOURCE_REQUEST_SOURCE_KINDS_ALL,
            )
        } else {
            webview.AddWebResourceRequestedFilter(&filter, COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL)
        };
        if let Err(e) = filtered {
            tracing::warn!("shield: filter failed: {e}");
            return;
        }
        let mut token = Default::default();
        let _ = webview.add_WebResourceRequested(
            &WebResourceRequestedEventHandler::create(Box::new(move |_, args| {
                let Some(args) = args else {
                    return Ok(());
                };
                let req = args.Request()?;
                let mut uri = PWSTR::null();
                req.Uri(&mut uri)?;
                let uri = take_pwstr(uri);
                if uri.contains("vapurr.localhost") || uri.starts_with("vapurr:") {
                    return Ok(());
                }
                let mut ctx = COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL;
                let _ = args.ResourceContext(&mut ctx);
                let page = page_url.lock().map(|s| s.clone()).unwrap_or_default();
                let rtype = resource_type_from_webview(ctx.0, &uri, &page);
                if !shield.should_block(&uri, &page, rtype) {
                    return Ok(());
                }
                let status = HSTRING::from("Blocked");
                let headers = HSTRING::from(
                    "Content-Type: text/plain\r\nAccess-Control-Allow-Origin: *\r\nCache-Control: no-store\r\n",
                );
                if let Ok(resp) = env.CreateWebResourceResponse(None, 204, &status, &headers) {
                    let _ = args.SetResponse(&resp);
                }
                Ok(())
            })),
            &mut token,
        );
    }
    tracing::info!("vapurr-shield attached");
}

/// Cancel OS-handler prompts for WalletConnect `wc:` URIs. The QR on fomo.family
/// is the real path (Robinhood Wallet is mobile). A Windows "open this app?"
/// dialog would sit on top of it and usually fail.
#[cfg(windows)]
pub fn attach_wallet_schemes(page: &wry::WebView) {
    use webview2_com::Microsoft::Web::WebView2::Win32::*;
    use webview2_com::{take_pwstr, LaunchingExternalUriSchemeEventHandler};
    use windows::Win32::Foundation::BOOL;
    use windows::core::{Interface, PWSTR};
    use wry::WebViewExtWindows;

    let controller = page.controller();
    unsafe {
        let Ok(webview) = controller.CoreWebView2() else {
            return;
        };
        let Ok(wv18) = webview.cast::<ICoreWebView2_18>() else {
            tracing::warn!("wallet schemes: no ICoreWebView2_18");
            return;
        };
        let mut token = Default::default();
        let _ = wv18.add_LaunchingExternalUriScheme(
            &LaunchingExternalUriSchemeEventHandler::create(Box::new(move |_, args| {
                let Some(args) = args else {
                    return Ok(());
                };
                let mut uri = PWSTR::null();
                args.Uri(&mut uri)?;
                let uri = take_pwstr(uri);
                if crate::host::is_wallet_scheme(&uri) {
                    let _ = args.SetCancel(BOOL(1));
                }
                Ok(())
            })),
            &mut token,
        );
    }
}

#[cfg(not(windows))]
pub fn attach(_page: &wry::WebView, _shield: Arc<Shield>, _page_url: Arc<std::sync::Mutex<String>>) {}

#[cfg(not(windows))]
pub fn attach_wallet_schemes(_page: &wry::WebView) {}
