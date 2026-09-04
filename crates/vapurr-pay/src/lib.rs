//! HTTP **402** is Payment Required. HTTP **404** is Not Found.
//! vapurr speaks x402 v2. We do not treat 404 as a paywall.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use vapurr_rhc::{self as rhc, format_usd_units, USDG};

pub const HEADER_REQUIRED: &str = "PAYMENT-REQUIRED";
pub const HEADER_SIGNATURE: &str = "PAYMENT-SIGNATURE";
pub const HEADER_RESPONSE: &str = "PAYMENT-RESPONSE";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentRequired {
    pub x402_version: u32,
    #[serde(default)]
    pub error: Option<String>,
    pub resource: Resource,
    pub accepts: Vec<Accept>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    pub url: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Accept {
    pub scheme: String,
    pub network: String,
    pub amount: String,
    pub asset: String,
    pub pay_to: String,
    #[serde(default)]
    pub max_timeout_seconds: Option<u64>,
    #[serde(default)]
    pub extra: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paywall {
    pub title: String,
    pub amount_label: String,
    pub amount_minor: u128,
    pub resource: String,
    pub required: PaymentRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopedCardAuth {
    pub merchant: String,
    pub max_cents: u64,
    pub expires: DateTime<Utc>,
    pub single_use: bool,
    pub collateral_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PayPlan {
    X402 { accept: Accept, amount_minor: u128 },
    CardPassthrough { auth: ScopedCardAuth },
    NeedKyc,
    NeedCardLink,
}

#[derive(Debug, Clone, Default)]
pub struct PayContext {
    pub verified: bool,
    pub card_linked: bool,
    pub card_collateral: Option<String>,
    pub prefer_card_for_fiat: bool,
}

pub struct PayRouter;

impl PayRouter {
    pub fn decide(required: &PaymentRequired, ctx: &PayContext) -> PayPlan {
        if !ctx.verified {
            return PayPlan::NeedKyc;
        }
        if let Some(accept) = pick_rhc_dollar(required) {
            let amount = parse_amount(&accept.amount);
            return PayPlan::X402 {
                accept,
                amount_minor: amount,
            };
        }
        if !ctx.card_linked {
            return PayPlan::NeedCardLink;
        }
        let amount = required
            .accepts
            .first()
            .map(|a| parse_amount(&a.amount))
            .unwrap_or(0);
        PayPlan::CardPassthrough {
            auth: ScopedCardAuth {
                merchant: required.resource.url.clone(),
                max_cents: (amount / 10_000) as u64,
                expires: Utc::now() + Duration::minutes(15),
                single_use: true,
                collateral_address: ctx.card_collateral.clone().unwrap_or_default(),
            },
        }
    }
}

fn is_pusd(asset: &str) -> bool {
    let a = asset.trim();
    if a.eq_ignore_ascii_case("PUSD") || a.eq_ignore_ascii_case("$PUSD") {
        return true;
    }
    [rhc::PUSD_TOKEN, rhc::TESTNET_PUSD]
        .iter()
        .any(|ca| !ca.is_empty() && a.eq_ignore_ascii_case(ca))
}

fn accept_decimals(a: &Accept) -> u8 {
    if let Some(d) = a.extra.get("decimals").and_then(|v| v.as_u64()) {
        return d.min(36) as u8;
    }
    if is_pusd(&a.asset) || a.asset.eq_ignore_ascii_case("VAPURR") {
        18
    } else {
        rhc::USDG_DECIMALS
    }
}

fn pick_rhc_dollar(required: &PaymentRequired) -> Option<Accept> {
    // v1.2: settle on testnet 46630 only. Do not pick mainnet 4663.
    let on_rhc = |a: &&Accept| {
        a.network == rhc::TESTNET_CAIP2 && (a.scheme == "exact" || a.scheme == "upto")
    };
    required
        .accepts
        .iter()
        .find(|a| on_rhc(a) && is_pusd(&a.asset))
        .cloned()
        .or_else(|| {
            required
                .accepts
                .iter()
                .find(|a| on_rhc(a) && a.asset.eq_ignore_ascii_case(USDG))
                .cloned()
        })
}

pub fn parse_amount(s: &str) -> u128 {
    s.trim().parse().unwrap_or(0)
}

pub fn is_paywall_status(status: u16) -> bool {
    status == 402
}

pub fn decode_required_header(value: &str) -> Result<PaymentRequired, PayError> {
    let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, value.trim())
        .map_err(|_| PayError::BadHeader)?;
    serde_json::from_slice(&bytes).map_err(|_| PayError::BadHeader)
}

pub fn encode_required_header(req: &PaymentRequired) -> Result<String, PayError> {
    let json = serde_json::to_vec(req).map_err(|_| PayError::BadHeader)?;
    Ok(base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        json,
    ))
}

pub fn paywall_from(
    status: u16,
    header: Option<&str>,
    body: Option<&str>,
    url: &str,
) -> Option<Paywall> {
    if status == 404 || !is_paywall_status(status) {
        return None;
    }
    let required = if let Some(h) = header {
        decode_required_header(h).ok()
    } else {
        None
    }
    .or_else(|| body.and_then(|b| serde_json::from_str(b).ok()))?;
    let accept = required.accepts.first();
    let amount = accept.map(|a| parse_amount(&a.amount)).unwrap_or(0);
    let decimals = accept.map(accept_decimals).unwrap_or(18);
    let resource = if required.resource.url.is_empty() {
        url.to_string()
    } else {
        required.resource.url.clone()
    };
    Some(Paywall {
        title: "Pay to continue".into(),
        amount_label: format_usd_units(amount, decimals),
        amount_minor: amount,
        resource,
        required,
    })
}

/// zzzmail postage. $0.0025 in $PUSD or $VAPURR. Scheme is a gasless voucher
/// so the sender does not pay ETH. Token units are 18 decimals.
pub const MAIL_POSTAGE_USD_MICROS: u128 = 2_500;
pub const MAIL_POSTAGE_TOKEN: u128 = 2_500_000_000_000_000;

pub fn mail_postage(pay_to: &str, asset: &str) -> PaymentRequired {
    let symbol = if asset.eq_ignore_ascii_case("vapurr") {
        "VAPURR"
    } else {
        "PUSD"
    };
    PaymentRequired {
        x402_version: 2,
        error: None,
        resource: Resource {
            url: "vapurr://zzzmail".into(),
            description: Some("zzzmail postage".into()),
            mime_type: Some("application/json".into()),
        },
        accepts: vec![Accept {
            scheme: "voucher".into(),
            network: rhc::TESTNET_CAIP2.into(),
            amount: MAIL_POSTAGE_TOKEN.to_string(),
            asset: symbol.into(),
            pay_to: pay_to.into(),
            max_timeout_seconds: Some(86_400),
            extra: serde_json::json!({
                "symbol": symbol,
                "decimals": 18,
                "token": if symbol == "VAPURR" { rhc::TESTNET_VAPURR } else { rhc::TESTNET_PUSD },
                "gasless": true,
                "usdMicros": MAIL_POSTAGE_USD_MICROS,
                "label": format!("0.25¢ ${symbol}"),
            }),
        }],
    }
}

pub fn home_accept(pay_to: &str, amount_minor: u128, url: &str) -> PaymentRequired {
    PaymentRequired {
        x402_version: 2,
        error: Some("PAYMENT-SIGNATURE header is required".into()),
        resource: Resource {
            url: url.into(),
            description: Some("protected resource".into()),
            mime_type: Some("text/html".into()),
        },
        accepts: vec![Accept {
            scheme: "exact".into(),
            network: rhc::TESTNET_CAIP2.into(),
            amount: amount_minor.to_string(),
            asset: "PUSD".into(),
            pay_to: pay_to.into(),
            max_timeout_seconds: Some(60),
            extra: serde_json::json!({
                "symbol": "PUSD",
                "decimals": 18,
                "token": rhc::TESTNET_PUSD,
            }),
        }],
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PayError {
    #[error("bad x402 header")]
    BadHeader,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_oh_four_is_not_a_paywall() {
        assert!(paywall_from(404, None, None, "https://x").is_none());
        assert!(!is_paywall_status(404));
        assert!(is_paywall_status(402));
    }

    #[test]
    fn header_roundtrip() {
        let req = home_accept(
            "0x0000000000000000000000000000000000000001",
            1_200_000_000_000_000_000,
            "https://api.example/premium",
        );
        let h = encode_required_header(&req).unwrap();
        let back = decode_required_header(&h).unwrap();
        assert_eq!(back.x402_version, 2);
        assert_eq!(back.accepts[0].network, rhc::TESTNET_CAIP2);
        assert_eq!(back.accepts[0].asset, "PUSD");
        assert_eq!(back.accepts[0].extra["token"], rhc::TESTNET_PUSD);
        let wall = paywall_from(402, Some(&h), None, "https://api.example/premium").unwrap();
        assert_eq!(wall.amount_label, "$1.20");
    }

    #[test]
    fn router_needs_kyc() {
        let req = home_accept("0xabc", 1000, "https://x");
        let plan = PayRouter::decide(&req, &PayContext::default());
        assert!(matches!(plan, PayPlan::NeedKyc));
    }

    #[test]
    fn mail_postage_is_gasless_and_under_a_cent() {
        let req = mail_postage("0xabc", "PUSD");
        assert_eq!(req.accepts[0].scheme, "voucher");
        assert_eq!(req.accepts[0].extra["gasless"], true);
        assert_eq!(req.accepts[0].extra["usdMicros"], 2_500);
        assert!(MAIL_POSTAGE_USD_MICROS < 10_000);
        let v = mail_postage("0xabc", "VAPURR");
        assert_eq!(v.accepts[0].asset, "VAPURR");
    }

    #[test]
    fn router_picks_x402_on_rhc() {
        let req = home_accept("0xabc", 1000, "https://x");
        let ctx = PayContext {
            verified: true,
            card_linked: true,
            card_collateral: Some("0xcard".into()),
            prefer_card_for_fiat: false,
        };
        match PayRouter::decide(&req, &ctx) {
            PayPlan::X402 { amount_minor, .. } => assert_eq!(amount_minor, 1000),
            other => panic!("expected x402, got {other:?}"),
        }
    }

    #[test]
    fn router_prefers_pusd_on_rhc() {
        let mut req = home_accept("0xabc", 1000, "https://x");
        req.accepts.insert(
            0,
            Accept {
                scheme: "exact".into(),
                network: rhc::TESTNET_CAIP2.into(),
                amount: "2000".into(),
                asset: "PUSD".into(),
                pay_to: "0xabc".into(),
                max_timeout_seconds: Some(60),
                extra: serde_json::json!({ "symbol": "PUSD", "decimals": 18 }),
            },
        );
        let ctx = PayContext {
            verified: true,
            card_linked: true,
            card_collateral: Some("0xcard".into()),
            prefer_card_for_fiat: false,
        };
        match PayRouter::decide(&req, &ctx) {
            PayPlan::X402 { accept, amount_minor } => {
                assert_eq!(accept.asset, "PUSD");
                assert_eq!(amount_minor, 2000);
            }
            other => panic!("expected PUSD x402, got {other:?}"),
        }
    }

    #[test]
    fn router_ignores_mainnet_dollar() {
        let mut req = home_accept("0xabc", 1000, "https://x");
        req.accepts[0].network = rhc::CAIP2.into();
        req.accepts[0].asset = USDG.into();
        let ctx = PayContext {
            verified: true,
            card_linked: true,
            card_collateral: Some("0xcard".into()),
            prefer_card_for_fiat: false,
        };
        match PayRouter::decide(&req, &ctx) {
            PayPlan::CardPassthrough { .. } => {}
            other => panic!("mainnet 4663 must not settle KetPay, got {other:?}"),
        }
    }
}
