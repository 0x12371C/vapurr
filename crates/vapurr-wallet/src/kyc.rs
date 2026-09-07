/* Wallet KYC attest helpers — challenge + EIP-191 personal_sign + issuer POST.
 * Age is a checkbox self-attest. Jurisdiction is CDN/IP geo on the issuer, not a dropdown.
 * No personal name is collected or stored.
 */
use serde_json::{json, Value};
use crate::{keccak256, DeviceKey, WalletError};

const ISSUER: &str = "https://thesecretlab.app";

fn http() -> Result<reqwest::blocking::Client, WalletError> {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(25))
        .user_agent("vapurr-desk-kyc/1")
        .build()
        .map_err(|e| WalletError::Fail(format!("http client: {e}")))
}

/// EIP-191 personal_sign — ethers verifyMessage compatible (v = 27/28).
pub fn personal_sign(key: &DeviceKey, message: &str) -> Result<String, WalletError> {
    let mut pref = format!("\x19Ethereum Signed Message:\n{}", message.len()).into_bytes();
    pref.extend_from_slice(message.as_bytes());
    let hash = keccak256(&pref);
    let mut sig = key.sign_digest(&hash)?;
    // sign_digest returns yParity 0/1; Ethereum personal_sign uses 27/28.
    if sig[64] < 27 {
        sig[64] += 27;
    }
    Ok(format!("0x{}", hex::encode(sig)))
}

fn challenge(client: &reqwest::blocking::Client) -> Result<(String, String), WalletError> {
    let v: Value = client
        .get(format!("{ISSUER}/api/kyc/challenge"))
        .send()
        .map_err(|e| WalletError::Fail(format!("challenge: {e}")))?
        .error_for_status()
        .map_err(|e| WalletError::Fail(format!("challenge: {e}")))?
        .json()
        .map_err(|e| WalletError::Fail(format!("challenge json: {e}")))?;
    let nonce = v
        .get("nonce")
        .and_then(|x| x.as_str())
        .ok_or_else(|| WalletError::Fail("challenge missing nonce".into()))?
        .to_string();
    let message = v
        .get("message")
        .and_then(|x| x.as_str())
        .ok_or_else(|| WalletError::Fail("challenge missing message".into()))?
        .to_string();
    Ok((nonce, message))
}

fn post_attest(client: &reqwest::blocking::Client, path: &str, body: Value) -> Result<Value, WalletError> {
    let resp = client
        .post(format!("{ISSUER}{path}"))
        .json(&body)
        .send()
        .map_err(|e| WalletError::Fail(format!("attest: {e}")))?;
    let status = resp.status();
    let v: Value = resp
        .json()
        .map_err(|e| WalletError::Fail(format!("attest json: {e}")))?;
    if !status.is_success() {
        let err = v
            .get("error")
            .and_then(|x| x.as_str())
            .unwrap_or("attestation failed");
        return Err(WalletError::Fail(err.into()));
    }
    Ok(v)
}

fn wrap_ok(level: u8, issuer: Value, extra: Value) -> Value {
    let mut out = json!({
        "ok": true,
        "kyc": true,
        "level": level,
        "logged_in": crate::session::is_logged_in(),
        "has_key": crate::session::has_key(),
        "has_pin": crate::session::has_pin(),
        "needs_pin": crate::session::needs_passcode_setup(),
        "address": issuer.get("address").cloned().unwrap_or(json!(crate::session::peek_address().unwrap_or_default())),
        "attestation": issuer.get("attestation").cloned().unwrap_or(Value::Null),
    });
    if let Some(obj) = out.as_object_mut() {
        if let Some(extra_obj) = extra.as_object() {
            for (k, v) in extra_obj {
                obj.insert(k.clone(), v.clone());
            }
        }
    }
    out
}

/// Level 1 — AgeOver18 self-attest (checkbox), wallet-signed. Not a DOB document.
pub fn attest_age(key: &DeviceKey, age_confirmed: bool) -> Result<Value, WalletError> {
    crate::session::require_unlocked()?;
    if !age_confirmed {
        return Err(WalletError::Fail(
            "Confirm you are 18 or older to continue (self-attest checkbox)".into(),
        ));
    }
    let client = http()?;
    let (nonce, message) = challenge(&client)?;
    let signature = personal_sign(key, &message)?;
    let issuer = post_attest(
        &client,
        "/api/kyc/attest/age",
        json!({
            "nonce": nonce,
            "signature": signature,
            "ageConfirmed": true,
        }),
    )?;
    Ok(wrap_ok(1, issuer, json!({})))
}

/// Level 2 — jurisdiction from issuer CDN/IP geo. Refuses embargoed countries.
pub fn attest_jurisdiction(key: &DeviceKey) -> Result<Value, WalletError> {
    crate::session::require_unlocked()?;
    let client = http()?;
    let (nonce, message) = challenge(&client)?;
    let signature = personal_sign(key, &message)?;
    let issuer = post_attest(
        &client,
        "/api/kyc/attest/jurisdiction",
        json!({
            "nonce": nonce,
            "signature": signature,
        }),
    )?;
    let country = issuer.get("country").cloned().unwrap_or(Value::Null);
    Ok(wrap_ok(2, issuer, json!({ "country": country })))
}

/// Sign an arbitrary UTF-8 message (EIP-191). Used by chrome pages that need wallet proof.
pub fn sign_message(key: &DeviceKey, message: &str) -> Result<Value, WalletError> {
    crate::session::require_unlocked()?;
    if message.is_empty() {
        return Err(WalletError::Fail("message required".into()));
    }
    if message.len() > 4096 {
        return Err(WalletError::Fail("message too long".into()));
    }
    let signature = personal_sign(key, message)?;
    Ok(json!({
        "ok": true,
        "kind": "signed",
        "signature": signature,
        "address": key.address.to_checksum(),
        "logged_in": crate::session::is_logged_in(),
        "has_key": crate::session::has_key(),
        "has_pin": crate::session::has_pin(),
        "needs_pin": crate::session::needs_passcode_setup(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use k256::ecdsa::SigningKey;
    use rand::rngs::OsRng;

    #[test]
    fn personal_sign_v_is_ethereum() {
        let signing = SigningKey::random(&mut OsRng);
        // Build a DeviceKey-like sign via raw — use DeviceKey::generate path.
        let key = DeviceKey::generate();
        let sig = personal_sign(&key, "zer0ID test").unwrap();
        assert!(sig.starts_with("0x"));
        assert_eq!(sig.len(), 132); // 0x + 65 bytes hex
        let v = u8::from_str_radix(&sig[sig.len() - 2..], 16).unwrap();
        assert!(v == 27 || v == 28, "v={v}");
        let _ = signing; // silence
    }
}
