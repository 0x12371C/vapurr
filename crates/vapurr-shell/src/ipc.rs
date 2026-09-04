
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
    CycleTab { back: bool },
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
    ShieldDom { ids: Vec<String>, classes: Vec<String> },
    Blobs,
    BlobSnap,
    Boost,
    Econ,
    EconMint(String),
    EconRedeem(String),
    EconDeploy,
    EconSeed { usdg: String, vapurr: String },
    EconSnap(serde_json::Value),
    EconErr { which: String, msg: String },
    Outbid,
    OutbidBid {
        url: String,
        title: String,
        amt: String,
    },
    OutbidDeploy,
    OutbidSnap(serde_json::Value),
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
    WalletImport {
        secret: String,
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
    ZzzmailHood { name: String },
}


pub(crate) fn parse_ipc(body: &str) -> Option<Msg> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    match v.get("cmd")?.as_str()? {
        "go" => Some(Msg::Go(v.get("url")?.as_str()?.to_string())),
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
            title: v.get("title").and_then(|x| x.as_str()).map(|s| s.to_string()),
        }),
        "cookies" => Some(Msg::Cookies),
        "cookie-del" => Some(Msg::CookieDel {
            name: v.get("name").and_then(|x| x.as_str()).unwrap_or("").into(),
            domain: v.get("domain").and_then(|x| x.as_str()).unwrap_or("").into(),
            path: v.get("path").and_then(|x| x.as_str()).unwrap_or("/").into(),
            host: v.get("host").and_then(|x| x.as_str()).map(|s| s.to_string()),
        }),
        "showfind" => Some(Msg::ShowFind),
        "find" => Some(Msg::Find(v.get("q").and_then(|x| x.as_str()).unwrap_or("").into())),
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
            asset: v.get("asset").and_then(|x| x.as_str()).unwrap_or("usdg").into(),
            to: v.get("to").and_then(|x| x.as_str()).unwrap_or("").into(),
            amt: v.get("amt").and_then(|x| x.as_str()).unwrap_or("").into(),
        }),
        "wallet-import" => Some(Msg::WalletImport {
            secret: v.get("secret").and_then(|x| x.as_str()).unwrap_or("").into(),
        }),
        "login-status" => Some(Msg::LoginStatus),
        "login-continue" => Some(Msg::LoginContinue),
        "login-create" => Some(Msg::LoginCreate),
        "login-restore" => Some(Msg::LoginRestore {
            secret: v.get("secret").and_then(|x| x.as_str()).unwrap_or("").into(),
        }),
        "logout" => Some(Msg::Logout),
        "patch-apply" => Some(Msg::PatchApply),
        "zzzmail-send" => Some(Msg::ZzzmailSend {
            to: v.get("to").and_then(|x| x.as_str()).unwrap_or("").into(),
            body: v.get("body").and_then(|x| x.as_str()).unwrap_or("").into(),
            asset: v.get("asset").and_then(|x| x.as_str()).unwrap_or("PUSD").into(),
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
            vapurr: v.get("vapurr").and_then(|x| x.as_str()).unwrap_or("").into(),
        }),
        "outbid" | "vapurrbid" => Some(Msg::Outbid),
        "outbid-bid" | "vapurrbid-bid" => Some(Msg::OutbidBid {
            url: v.get("url").and_then(|x| x.as_str()).unwrap_or("").into(),
            title: v.get("title").and_then(|x| x.as_str()).unwrap_or("").into(),
            amt: v.get("amt").and_then(|x| x.as_str()).unwrap_or("").into(),
        }),
        "outbid-deploy" | "vapurrbid-deploy" => Some(Msg::OutbidDeploy),
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

