use aes_gcm::{
    Aes128Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use anyhow::{Result, anyhow};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use hkdf::Hkdf;
use p256::{
    EncodedPoint, PublicKey,
    ecdh::EphemeralSecret,
    ecdsa::{Signature, SigningKey, signature::Signer},
};
use rand::rngs::OsRng;
use sha2::Sha256;

use crate::db::Subscription;

pub struct VapidConfig {
    pub subject: String,
    pub public_key_b64: String,
    pub private_key_b64: String,
}

impl VapidConfig {
    pub fn configured(&self) -> bool {
        !self.public_key_b64.is_empty() && !self.private_key_b64.is_empty()
    }
}

pub async fn send_all(
    subscriptions: &[Subscription],
    message: &str,
    vapid: &VapidConfig,
) -> Vec<Result<()>> {
    let client = reqwest::Client::builder()
        .use_rustls_tls()
        .build()
        .expect("reqwest client");

    let mut results = Vec::new();
    for sub in subscriptions {
        results.push(send_one(&client, sub, message, vapid).await);
    }
    results
}

pub async fn send_one_sub(sub: &Subscription, message: &str, vapid: &VapidConfig) -> Result<()> {
    let client = reqwest::Client::builder()
        .use_rustls_tls()
        .build()
        .expect("reqwest client");

    send_one(&client, sub, message, vapid).await
}

async fn send_one(
    client: &reqwest::Client,
    sub: &Subscription,
    message: &str,
    vapid: &VapidConfig,
) -> Result<()> {
    let p256dh_bytes = URL_SAFE_NO_PAD.decode(&sub.p256dh)?;
    let auth_bytes = URL_SAFE_NO_PAD.decode(&sub.auth)?;
    let ua_pubkey =
        PublicKey::from_sec1_bytes(&p256dh_bytes).map_err(|e| anyhow!("invalid p256dh: {e}"))?;

    let payload = encrypt_payload(message.as_bytes(), &ua_pubkey, &auth_bytes)?;
    let origin = extract_origin(&sub.endpoint)?;
    let jwt = build_vapid_jwt(&origin, &vapid.subject, &vapid.private_key_b64)?;
    let auth_header = format!("vapid t={jwt}, k={}", vapid.public_key_b64);

    let response = client
        .post(sub.endpoint.as_str())
        .header("Content-Type", "application/octet-stream")
        .header("Content-Encoding", "aes128gcm")
        .header("Authorization", &auth_header)
        .header("TTL", "43200")
        .header("Urgency", "high")
        .body(payload)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("push endpoint returned {status}: {body}");
    }

    Ok(())
}

fn encrypt_payload(plaintext: &[u8], ua_pubkey: &PublicKey, auth_secret: &[u8]) -> Result<Vec<u8>> {
    let server_secret = EphemeralSecret::random(&mut OsRng);
    let server_pubkey = server_secret.public_key();
    let server_pubkey_bytes = EncodedPoint::from(&server_pubkey).to_bytes().to_vec();
    let ua_ep = EncodedPoint::from(ua_pubkey);
    let shared = server_secret.diffie_hellman(ua_pubkey);
    let shared_bytes = shared.raw_secret_bytes();
    let ua_pubkey_bytes = ua_ep.to_bytes();

    let mut info = b"WebPush: info\x00".to_vec();
    info.extend_from_slice(&ua_pubkey_bytes);
    info.extend_from_slice(&server_pubkey_bytes);

    let hk = Hkdf::<Sha256>::new(Some(auth_secret), shared_bytes.as_slice());
    let mut ikm = [0u8; 32];
    hk.expand(&info, &mut ikm)
        .map_err(|_| anyhow!("HKDF expand failed for IKM"))?;

    let mut salt = [0u8; 16];
    rand::RngCore::fill_bytes(&mut OsRng, &mut salt);

    let hk2 = Hkdf::<Sha256>::new(Some(&salt), &ikm);
    let mut cek = [0u8; 16];
    hk2.expand(b"Content-Encoding: aes128gcm\x00", &mut cek)
        .map_err(|_| anyhow!("HKDF expand failed for CEK"))?;

    let mut nonce_bytes = [0u8; 12];
    hk2.expand(b"Content-Encoding: nonce\x00", &mut nonce_bytes)
        .map_err(|_| anyhow!("HKDF expand failed for nonce"))?;

    let mut message = plaintext.to_vec();
    message.push(0x02);

    let cipher = Aes128Gcm::new_from_slice(&cek)?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, message.as_slice())
        .map_err(|_| anyhow!("AES-GCM encryption failed"))?;

    let record_size: u32 = 4096;
    let mut payload = Vec::new();
    payload.extend_from_slice(&salt);
    payload.extend_from_slice(&record_size.to_be_bytes());
    payload.push(server_pubkey_bytes.len() as u8);
    payload.extend_from_slice(&server_pubkey_bytes);
    payload.extend_from_slice(&ciphertext);

    Ok(payload)
}

fn build_vapid_jwt(audience: &str, subject: &str, private_key_b64: &str) -> Result<String> {
    let header = serde_json::json!({"alg": "ES256"});
    let header_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_string(&header)?);
    let exp = Utc::now().timestamp() + 3600;
    let payload = serde_json::json!({
        "aud": audience,
        "exp": exp,
        "sub": subject,
    });
    let payload_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_string(&payload)?);
    let signing_input = format!("{header_b64}.{payload_b64}");

    let private_key_bytes = URL_SAFE_NO_PAD.decode(private_key_b64)?;
    let signing_key = SigningKey::from_bytes(private_key_bytes.as_slice().into())
        .map_err(|e| anyhow!("invalid VAPID private key: {e}"))?;
    let signature: Signature = signing_key.sign(signing_input.as_bytes());
    let signature_b64 = URL_SAFE_NO_PAD.encode(signature.to_bytes());

    Ok(format!("{signing_input}.{signature_b64}"))
}

fn extract_origin(endpoint: &str) -> Result<String> {
    let url = reqwest::Url::parse(endpoint)?;
    let origin = format!(
        "{}://{}",
        url.scheme(),
        url.host_str()
            .ok_or_else(|| anyhow!("no host in endpoint URL"))?
    );

    if let Some(port) = url.port() {
        Ok(format!("{origin}:{port}"))
    } else {
        Ok(origin)
    }
}
