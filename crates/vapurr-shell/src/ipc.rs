#[derive(Clone)]
pub(crate) enum Msg {
    Go(String),
    Home,
    Back,
    Forward,
    Reload,
    Pane(String),
    Url(String),
    Title(String),
    Chain(String),
    NewTab,
    CloseTab(Option<u64>),
    SelectTab(u64),
    SelectTabAt(u64),
    CycleTab {
        back: bool,
    },
    Star {
        url: Option<String>,
        title: Option<String>,
    },
    Cookies,
    CookieDel {
        name: String,
        domain: String,
        path: String,
        host: Option<String>,
    },
    ShowFind,
    Find(String),
    FocusUrl,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    EarnToggle,
    EarnSubmit,
    Desk,
    ZoomSet(f64),
    Pref(String, serde_json::Value),
    Clear(String),
    PageStart(String),
    ShieldDom {
        ids: Vec<String>,
        classes: Vec<String>,
    },
    Blobs,
    BlobSnap,
    Boost,
    Econ,
    EconMint(String),
    EconRedeem(String),
    EconDeploy,
    EconSeed {
        usdg: String,
        vapurr: String,
    },
    EconSnap(serde_json::Value),
    EconErr {
        which: String,
        msg: String,
    },
    Outbid,
    OutbidBid {
        url: String,
        title: String,
        amt: String,
    },
    OutbidDeploy,
    OutbidSnap(serde_json::Value),
    KetList,
    KetListPay {
        token: String,
        pool: String,
        symbol: String,
        name: String,
        amt: String,
        meta: String,
    },
    KetListDeploy,
    KetListSnap(serde_json::Value),
    LoopDeploy,
    LoopOp {
        op: String,
        amt: String,
        steps: String,
    },
    HouseDeploy,
    HouseSeed {
        vapurr: String,
        pusd: String,
    },
    HouseBootstrap,
    HouseSwap {
        sell_v: bool,
        amt: String,
    },
    RadioLayout {
        float: bool,
        corner: String,
        collapsed: bool,
    },
    Wallet,
    WalletSend {
        asset: String,
        to: String,
        amt: String,
    },
    WalletExec {
        to: String,
        data: String,
        value: String,
        chain_id: u64,
        gas: u64,
    },
    WalletImport {
        secret: String,
    },
    WalletSetNet(String),
    WalletRevealSeed,
    WalletExportKey,
    WalletResolve {
        to: String,
    },
    LoginStatus,
    LoginContinue,
    LoginCreate,
    LoginRestore {
        secret: String,
    },
    Logout,
    PatchApply,
    WalletSnap(serde_json::Value),
    WalletErr(String),
    ZzzmailSend {
        to: String,
        body: String,
        asset: String,
    },
    ZzzmailInbox,
    ZzzmailHood {
        name: String,
    },
}

pub(crate) fn parse_ipc(body: &str) -> Option<Msg> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    match v.get("cmd")?.as_str()? {
        "go" | "open" => Some(Msg::Go(v.get("url")?.as_str()?.to_string())),
        "home" => Some(Msg::Home),
        "back" => Some(Msg::Back),
        "forward" => Some(Msg::Forward),
        "reload" => Some(Msg::Reload),
        "pane" => Some(Msg::Pane(v.get("id")?.as_str()?.to_string())),
        "newtab" => Some(Msg::NewTab),
        "closetab" => Some(Msg::CloseTab(v.get("id").and_then(|x| x.as_u64()))),
        "selecttab" => Some(Msg::SelectTab(v.get("id")?.as_u64()?)),
        "selecttabi" => Some(Msg::SelectTabAt(v.get("i")?.as_u64()?)),
        "nexttab" => Some(Msg::CycleTab { back: false }),
        "prevtab" => Some(Msg::CycleTab { back: true }),
        "star" => Some(Msg::Star {
            url: v.get("url").and_then(|x| x.as_str()).map(|s| s.to_string()),
            title: v
                .get("title")
                .and_then(|x| x.as_str())
                .map(|s| s.to_string()),
        }),
        "cookies" => Some(Msg::Cookies),
        "cookie-del" => Some(Msg::CookieDel {
            name: v.get("name").and_then(|x| x.as_str()).unwrap_or("").into(),
            domain: v
                .get("domain")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .into(),
            path: v.get("path").and_then(|x| x.as_str()).unwrap_or("/").into(),
            host: v
                .get("host")
                .and_then(|x| x.as_str())
                .map(|s| s.to_string()),
        }),
        "showfind" => Some(Msg::ShowFind),
        "find" => Some(Msg::Find(
            v.get("q").and_then(|x| x.as_str()).unwrap_or("").into(),
        )),
        "focusurl" => Some(Msg::FocusUrl),
        "zoomin" => Some(Msg::ZoomIn),
        "zoomout" => Some(Msg::ZoomOut),
        "zoomreset" => Some(Msg::ZoomReset),
        "earn-toggle" => Some(Msg::EarnToggle),
        "earn-submit" => Some(Msg::EarnSubmit),
        "desk" => Some(Msg::Desk),
        "zoomset" => Some(Msg::ZoomSet(v.get("factor")?.as_f64()?)),
        "pref" => Some(Msg::Pref(
            v.get("key")?.as_str()?.to_string(),
            v.get("value").cloned().unwrap_or(serde_json::Value::Null),
        )),
        "clear" => Some(Msg::Clear(v.get("what")?.as_str()?.to_string())),
        "settings" => Some(Msg::Pane("settings".into())),
        "shield-dom" => Some(Msg::ShieldDom {
            ids: json_str_vec(v.get("ids")),
            classes: json_str_vec(v.get("classes")),
        }),
        "blobs" => Some(Msg::Blobs),
        "blob-snap" => Some(Msg::BlobSnap),
        "boost" => Some(Msg::Boost),
        "wallet" => Some(Msg::Wallet),
        "wallet-send" => Some(Msg::WalletSend {
            asset: v
                .get("asset")
                .and_then(|x| x.as_str())
                .unwrap_or("usdg")
                .into(),
            to: v.get("to").and_then(|x| x.as_str()).unwrap_or("").into(),
            amt: v.get("amt").and_then(|x| x.as_str()).unwrap_or("").into(),
        }),
        "wallet-exec" => Some(Msg::WalletExec {
            to: v.get("to").and_then(|x| x.as_str()).unwrap_or("").into(),
            data: v.get("data").and_then(|x| x.as_str()).unwrap_or("").into(),
            value: v.get("value").and_then(|x| x.as_str()).unwrap_or("0x0").into(),
            chain_id: v.get("chain_id").and_then(|x| x.as_u64()).unwrap_or(0),
            gas: v.get("gas").and_then(|x| x.as_u64()).unwrap_or(0),
        }),
        "wallet-import" => Some(Msg::WalletImport {
            secret: v
                .get("secret")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .into(),
        }),
        "wallet-set-net" => Some(Msg::WalletSetNet(
            v.get("net")
                .and_then(|x| x.as_str())
                .unwrap_or("testnet")
                .into(),
        )),
        "wallet-reveal-seed" => Some(Msg::WalletRevealSeed),
        "wallet-export-key" => Some(Msg::WalletExportKey),
        "wallet-resolve" => Some(Msg::WalletResolve {
            to: v.get("to").and_then(|x| x.as_str()).unwrap_or("").into(),
        }),
        "login-status" => Some(Msg::LoginStatus),
        "login-continue" => Some(Msg::LoginContinue),
        "login-create" => Some(Msg::LoginCreate),
        "login-restore" => Some(Msg::LoginRestore {
            secret: v
                .get("secret")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .into(),
        }),
        "logout" => Some(Msg::Logout),
        "patch-apply" => Some(Msg::PatchApply),
        "zzzmail-send" => Some(Msg::ZzzmailSend {
            to: v.get("to").and_then(|x| x.as_str()).unwrap_or("").into(),
            body: v.get("body").and_then(|x| x.as_str()).unwrap_or("").into(),
            asset: v
                .get("asset")
                .and_then(|x| x.as_str())
                .unwrap_or("PUSD")
                .into(),
        }),
        "zzzmail-inbox" => Some(Msg::ZzzmailInbox),
        "zzzmail-hood" => Some(Msg::ZzzmailHood {
            name: v.get("name").and_then(|x| x.as_str()).unwrap_or("").into(),
        }),
        "econ" => Some(Msg::Econ),
        "econ-mint" => Some(Msg::EconMint(
            v.get("amt").and_then(|x| x.as_str()).unwrap_or("").into(),
        )),
        "econ-redeem" => Some(Msg::EconRedeem(
            v.get("amt").and_then(|x| x.as_str()).unwrap_or("").into(),
        )),
        "econ-deploy" => Some(Msg::EconDeploy),
        "econ-seed" => Some(Msg::EconSeed {
            usdg: v.get("usdg").and_then(|x| x.as_str()).unwrap_or("").into(),
            vapurr: v
                .get("vapurr")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .into(),
        }),
        "outbid" | "vapurrbid" => Some(Msg::Outbid),
        "outbid-bid" | "vapurrbid-bid" => Some(Msg::OutbidBid {
            url: v.get("url").and_then(|x| x.as_str()).unwrap_or("").into(),
            title: v.get("title").and_then(|x| x.as_str()).unwrap_or("").into(),
            amt: v.get("amt").and_then(|x| x.as_str()).unwrap_or("").into(),
        }),
        "outbid-deploy" | "vapurrbid-deploy" => Some(Msg::OutbidDeploy),
        "ketlist" | "ketcharts-list" => Some(Msg::KetList),
        "ketlist-pay" | "ketcharts-list-pay" => Some(Msg::KetListPay {
            token: v.get("token").and_then(|x| x.as_str()).unwrap_or("").into(),
            pool: v.get("pool").and_then(|x| x.as_str()).unwrap_or("").into(),
            symbol: v
                .get("symbol")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .into(),
            name: v.get("name").and_then(|x| x.as_str()).unwrap_or("").into(),
            amt: v.get("amt").and_then(|x| x.as_str()).unwrap_or("").into(),
            meta: v.get("meta").and_then(|x| x.as_str()).unwrap_or("").into(),
        }),
        "ketlist-deploy" | "ketcharts-list-deploy" => Some(Msg::KetListDeploy),
        "econ-loop-deploy" => Some(Msg::LoopDeploy),
        "econ-house-deploy" => Some(Msg::HouseDeploy),
        "econ-house-seed" => Some(Msg::HouseSeed {
            vapurr: v.get("vapurr").and_then(|x| x.as_str()).unwrap_or("").into(),
            pusd: v.get("pusd").and_then(|x| x.as_str()).unwrap_or("").into(),
        }),
        "econ-house-bootstrap" => Some(Msg::HouseBootstrap),
        "econ-swap" | "econ-house-swap" => Some(Msg::HouseSwap {
            sell_v: v.get("sell_v").and_then(|x| x.as_bool()).unwrap_or(true),
            amt: v.get("amt").and_then(|x| x.as_str()).unwrap_or("").into(),
        }),
        "econ-loop" => Some(Msg::LoopOp {
            op: v.get("op").and_then(|x| x.as_str()).unwrap_or("").into(),
            amt: v.get("amt").and_then(|x| x.as_str()).unwrap_or("").into(),
            steps: v
                .get("steps")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .into(),
        }),
        "radio-layout" => Some(Msg::RadioLayout {
            float: v.get("mode").and_then(|x| x.as_str()) == Some("float"),
            corner: v
                .get("corner")
                .and_then(|x| x.as_str())
                .unwrap_or("br")
                .to_string(),
            collapsed: v
                .get("collapsed")
                .and_then(|x| x.as_bool())
                .unwrap_or(false),
        }),
        _ => None,
    }
}

pub(crate) fn json_str_vec(v: Option<&serde_json::Value>) -> Vec<String> {
    v.and_then(|x| x.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .take(200)
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_cmd_is_go() {
        let m =
            parse_ipc(r#"{"cmd":"open","url":"https://www.thesecretlab.app/kyc"}"#).expect("open");
        match m {
            Msg::Go(u) => assert_eq!(u, "https://www.thesecretlab.app/kyc"),
            _ => panic!("open must navigate"),
        }
    }

    #[test]
    fn go_cmd_still_works() {
        let m = parse_ipc(r#"{"cmd":"go","url":"vapurr://ketpay"}"#).expect("go");
        match m {
            Msg::Go(u) => assert_eq!(u, "vapurr://ketpay"),
            _ => panic!("go"),
        }
    }

    #[test]
    fn wallet_exec_cmd() {
        match parse_ipc(
            r#"{"cmd":"wallet-exec","to":"0xabc","data":"0x12","value":"0x0","chain_id":4663,"gas":21000}"#,
        )
        .expect("exec")
        {
            Msg::WalletExec {
                to,
                chain_id,
                gas,
                ..
            } => {
                assert_eq!(to, "0xabc");
                assert_eq!(chain_id, 4663);
                assert_eq!(gas, 21000);
            }
            _ => panic!("exec"),
        }
    }

    #[test]
    fn wallet_settings_cmds() {
        match parse_ipc(r#"{"cmd":"wallet-set-net","net":"mainnet"}"#).expect("net") {
            Msg::WalletSetNet(n) => assert_eq!(n, "mainnet"),
            _ => panic!("set-net"),
        }
        assert!(matches!(
            parse_ipc(r#"{"cmd":"wallet-reveal-seed"}"#),
            Some(Msg::WalletRevealSeed)
        ));
        assert!(matches!(
            parse_ipc(r#"{"cmd":"wallet-export-key"}"#),
            Some(Msg::WalletExportKey)
        ));
        match parse_ipc(r#"{"cmd":"wallet-resolve","to":"relic.hood"}"#).expect("resolve") {
            Msg::WalletResolve { to } => assert_eq!(to, "relic.hood"),
            _ => panic!("resolve"),
        }
    }

    #[test]
    fn ketlist_cmds() {
        assert!(matches!(parse_ipc(r#"{"cmd":"ketlist"}"#), Some(Msg::KetList)));
        match parse_ipc(
            r#"{"cmd":"ketlist-pay","token":"0x11","pool":"0x22","symbol":"FOO","name":"Foo","amt":"50"}"#,
        )
        .expect("pay")
        {
            Msg::KetListPay {
                token,
                pool,
                symbol,
                name,
                amt,
                meta,
            } => {
                assert_eq!(token, "0x11");
                assert_eq!(pool, "0x22");
                assert_eq!(symbol, "FOO");
                assert_eq!(name, "Foo");
                assert_eq!(amt, "50");
                assert_eq!(meta, "");
            }
            _ => panic!("pay"),
        }
        assert!(matches!(
            parse_ipc(r#"{"cmd":"ketcharts-list-deploy"}"#),
            Some(Msg::KetListDeploy)
        ));
    }

    #[test]
    fn house_swap_cmd() {
        match parse_ipc(r#"{"cmd":"econ-swap","sell_v":false,"amt":"8"}"#).expect("swap") {
            Msg::HouseSwap { sell_v, amt } => {
                assert!(!sell_v);
                assert_eq!(amt, "8");
            }
            _ => panic!("swap"),
        }
    }
}
