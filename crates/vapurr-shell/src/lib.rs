//! Desk, tabs, and chrome helpers. The window lives in the `vapurr` binary.

pub mod desk;
pub mod tabs;

#[cfg(test)]
mod pusd_guard {
    #[test]
    fn pusd_is_the_mint_desk() {
        let html = include_str!("../../../frontend/pusd.html");
        assert!(html.contains("data-mode=\"mint\""), "one book, mint/redeem");
        assert!(html.contains("id=\"lithe\""), "Lithe lives on this desk");
        assert!(
            !html.contains("globe.js"),
            "PUSD is a money desk, not the globe clone"
        );
        let low = html.to_ascii_lowercase();
        assert!(!low.contains("terra"), "no Terra in PUSD copy");
        assert!(!low.contains("anchor"), "no Anchor in PUSD copy");
    }
}

#[cfg(test)]
mod liq_guard {
    #[test]
    fn explorer_liq_is_native_svg() {
        let js = include_str!("../../../frontend/explorer.js");
        assert!(js.contains("function drawLiqGraph"), "Scan must paint the map as SVG");
        assert!(
            !js.contains("vis-network"),
            "vis-network must not come back — it paints 0×0 in WebView2"
        );
        assert!(
            !js.contains("url(#liq"),
            "SVG url(#liq-…) hijacks #/liq and dumps the page"
        );
        assert!(
            !js.to_ascii_lowercase().contains("gecko"),
            "market book is Rust RPC, not a third-party HTTP feed"
        );
    }

    /// What Scan actually does when you click Explorer: PNS reverse map + head, then Tokens.
    /// wry's custom protocol runs this on the UI thread.
    #[test]
    fn explorer_boot_does_not_block() {
        use std::time::Instant;
        fn check(name: &str, body: impl Fn() -> String) {
            let t0 = Instant::now();
            let body = body();
            let ms = t0.elapsed().as_millis();
            eprintln!("boot {name}: {ms}ms {}B", body.len());
            assert!(
                ms < 250,
                "{name} took {ms}ms; custom protocol would lock the chrome. body={}",
                body.chars().take(160).collect::<String>()
            );
        }
        check("scan/head", || vapurr_rhc::scan::api("head", ""));
        check("scan/tokens", || vapurr_rhc::scan::api("tokens", ""));
        check("scan/txs", || vapurr_rhc::scan::api("txs", ""));
        check("scan/blocks", || vapurr_rhc::scan::api("blocks", ""));
        check("scan/gas", || vapurr_rhc::scan::api("gas", ""));
        check("scan/liq", || vapurr_rhc::scan::api("liq", ""));
        check("scan/token-usdg", || {
            vapurr_rhc::scan::api(
                "token",
                "a=0x5fc5360D0400a0Fd4f2af552ADD042D716F1d168",
            )
        });
        check("zzzmail/pns-status", || {
            vapurr_zmail::chain::status_snapshot("0x0000000000000000000000000000000000000001")
                .to_string()
        });
    }
}
