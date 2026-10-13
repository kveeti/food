use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::{RngCore, rngs::OsRng};
use rsa::{RsaPrivateKey, RsaPublicKey, pkcs1v15::Pkcs1v15Sign, traits::PublicKeyParts};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{
        Router, StatusCode,
        content::{Form, Json},
        error::bad_request,
        query_params,
        response::{IntoResponse, Response},
        route,
    },
    view::view,
};
use url::Url;

struct Provider {
    issuer: String,
    app_url: String,
    client_id: String,
    client_secret: String,
    access_lifetime: u64,
    refresh_delay: Duration,
    key: RsaPrivateKey,
    codes: Mutex<HashMap<String, PendingCode>>,
    refresh_tokens: Mutex<HashMap<String, PendingRefresh>>,
    sessions: Mutex<HashMap<String, Identity>>,
    http: reqwest::Client,
}

struct PendingCode {
    identity: Identity,
    session_id: String,
    nonce: String,
    code_challenge: String,
    redirect_uri: String,
    created_at: Instant,
}

struct PendingRefresh {
    identity: Identity,
    session_id: String,
    expires_at: Instant,
}

#[derive(Clone)]
struct Identity {
    subject: String,
    email: String,
}

#[topcoat::router::query_params(error = bad_request)]
struct AuthorizeQuery {
    client_id: String,
    redirect_uri: String,
    response_type: String,
    scope: String,
    state: String,
    nonce: String,
    code_challenge: String,
    code_challenge_method: String,
    sub: Option<String>,
}

#[derive(Deserialize)]
struct TokenForm {
    grant_type: String,
    code: Option<String>,
    redirect_uri: Option<String>,
    client_id: String,
    client_secret: String,
    code_verifier: Option<String>,
    refresh_token: Option<String>,
}

#[derive(Deserialize)]
struct RevokeForm {
    subject: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let provider = Provider {
        issuer: required("OIDC_ISSUER")?.trim_end_matches('/').to_owned(),
        app_url: required("APP_URL")?.trim_end_matches('/').to_owned(),
        client_id: required("OIDC_CLIENT_ID")?,
        client_secret: required("OIDC_CLIENT_SECRET")?,
        access_lifetime: optional_number("IDP_ACCESS_TOKEN_LIFETIME", 300)?,
        refresh_delay: Duration::from_millis(optional_number("IDP_REFRESH_DELAY", 0)?),
        key: RsaPrivateKey::new(&mut OsRng, 2048)?,
        codes: Mutex::new(HashMap::new()),
        refresh_tokens: Mutex::new(HashMap::new()),
        sessions: Mutex::new(HashMap::new()),
        http: reqwest::Client::new(),
    };

    topcoat::start(
        Router::builder()
            .route(discovery)
            .route(jwks)
            .route(authorize)
            .route(token)
            .route(revoke)
            .app_context(provider)
            .build(),
    )
    .await?;
    Ok(())
}

#[route(GET "/.well-known/openid-configuration")]
async fn discovery(cx: &Cx) -> Result<Json<Value>> {
    let issuer = &provider(cx).issuer;
    Ok(Json(json!({
        "issuer": issuer,
        "authorization_endpoint": format!("{issuer}/authorize"),
        "token_endpoint": format!("{issuer}/token"),
        "jwks_uri": format!("{issuer}/jwks.json"),
        "response_types_supported": ["code"],
        "subject_types_supported": ["public"],
        "id_token_signing_alg_values_supported": ["RS256"],
        "scopes_supported": ["openid", "email"],
        "grant_types_supported": ["authorization_code", "refresh_token"],
        "token_endpoint_auth_methods_supported": ["client_secret_post"],
        "code_challenge_methods_supported": ["S256"]
    })))
}

#[route(GET "/jwks.json")]
async fn jwks(cx: &Cx) -> Result<Json<Value>> {
    let key = RsaPublicKey::from(&provider(cx).key);
    Ok(Json(json!({
        "keys": [{
            "kty": "RSA",
            "use": "sig",
            "alg": "RS256",
            "kid": "dev",
            "n": URL_SAFE_NO_PAD.encode(key.n().to_bytes_be()),
            "e": URL_SAFE_NO_PAD.encode(key.e().to_bytes_be())
        }]
    })))
}

#[route(GET "/authorize")]
async fn authorize(cx: &Cx) -> Result<Response> {
    let query = query_params::<AuthorizeQuery>(cx)?;
    let provider = provider(cx);
    if query.client_id != provider.client_id
        || query.redirect_uri != format!("{}/auth/callback", provider.app_url)
        || query.response_type != "code"
        || !query.scope.split(' ').any(|scope| scope == "openid")
        || query.code_challenge_method != "S256"
        || query.state.is_empty()
        || query.nonce.is_empty()
    {
        return Err(bad_request("invalid OIDC authorization request").into());
    }

    if let Some(subject) = query.sub.as_deref() {
        let identity = Identity {
            subject: subject.to_owned(),
            email: format!("{subject}@dev.local"),
        };
        let session_id = random_token();
        let code = random_token();
        provider
            .sessions
            .lock()
            .expect("dev OIDC session lock poisoned")
            .insert(session_id.clone(), identity.clone());
        provider
            .codes
            .lock()
            .expect("dev OIDC code lock poisoned")
            .insert(
                code.clone(),
                PendingCode {
                    identity,
                    session_id,
                    nonce: query.nonce.clone(),
                    code_challenge: query.code_challenge.clone(),
                    redirect_uri: query.redirect_uri.clone(),
                    created_at: Instant::now(),
                },
            );
        let mut callback = Url::parse(&query.redirect_uri)?;
        callback
            .query_pairs_mut()
            .append_pair("code", &code)
            .append_pair("state", &query.state);
        return redirect(callback.as_str(), cx);
    }

    let alice = authorize_url(provider, query, "alice")?;
    let bob = authorize_url(provider, query, "bob")?;
    view! {
        <!DOCTYPE html>
        <html lang="en" style="color-scheme: light dark">
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <title>"Dev login"</title>
            </head>
            <body>
                <main>
                    <h1>"Pick a dev user"</h1>
                    <p><a href=(alice)>"alice - alice@dev.local"</a></p>
                    <p><a href=(bob)>"bob - bob@dev.local"</a></p>
                </main>
            </body>
        </html>
    }?
    .into_response(cx)
}

#[route(POST "/token")]
async fn token(cx: &Cx, Form(form): Form<TokenForm>) -> Result<Response> {
    let provider = provider(cx);
    if form.client_id != provider.client_id || form.client_secret != provider.client_secret {
        return oauth_error("invalid_client", cx);
    }

    match form.grant_type.as_str() {
        "authorization_code" => {
            let Some(code) = form.code else {
                return oauth_error("invalid_grant", cx);
            };
            let Some(pending) = provider
                .codes
                .lock()
                .expect("dev OIDC code lock poisoned")
                .remove(&code)
            else {
                return oauth_error("invalid_grant", cx);
            };
            let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(
                form.code_verifier.as_deref().unwrap_or_default().as_bytes(),
            ));
            if pending.created_at.elapsed() >= Duration::from_secs(600)
                || form.redirect_uri.as_deref() != Some(pending.redirect_uri.as_str())
                || challenge != pending.code_challenge
            {
                return oauth_error("invalid_grant", cx);
            }
            Json(issue_tokens(
                provider,
                &pending.identity,
                &pending.session_id,
                Some(&pending.nonce),
            )?)
            .into_response(cx)
        }
        "refresh_token" => {
            tokio::time::sleep(provider.refresh_delay).await;
            let refresh_token = form.refresh_token.unwrap_or_default();
            let pending = provider
                .refresh_tokens
                .lock()
                .expect("dev OIDC refresh lock poisoned")
                .remove(&refresh_token);
            let Some(pending) = pending.filter(|pending| pending.expires_at > Instant::now())
            else {
                return oauth_error("invalid_grant", cx);
            };
            Json(issue_tokens(
                provider,
                &pending.identity,
                &pending.session_id,
                None,
            )?)
            .into_response(cx)
        }
        _ => oauth_error("unsupported_grant_type", cx),
    }
}

#[route(POST "/revoke")]
async fn revoke(cx: &Cx, Form(form): Form<RevokeForm>) -> Result<Response> {
    let sessions: Vec<_> = provider(cx)
        .sessions
        .lock()
        .expect("dev OIDC session lock poisoned")
        .iter()
        .filter(|(_, identity)| identity.subject == form.subject)
        .map(|(session_id, _)| session_id.clone())
        .collect();
    if sessions.is_empty() {
        return Err(bad_request("OIDC session not found").into());
    }

    for session_id in sessions {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let logout_token = sign(
            provider(cx),
            json!({
                "iss": provider(cx).issuer,
                "aud": provider(cx).client_id,
                "iat": now,
                "jti": random_token(),
                "sid": session_id,
                "events": {
                    "http://schemas.openid.net/event/backchannel-logout": {}
                }
            }),
            "logout+jwt",
        )?;
        let response = provider(cx)
            .http
            .post(format!("{}/auth/backchannel-logout", provider(cx).app_url))
            .form(&[("logout_token", logout_token)])
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("back-channel logout failed").into());
        }
        provider(cx)
            .sessions
            .lock()
            .expect("dev OIDC session lock poisoned")
            .remove(&session_id);
        provider(cx)
            .refresh_tokens
            .lock()
            .expect("dev OIDC refresh lock poisoned")
            .retain(|_, refresh| refresh.session_id != session_id);
    }

    ().into_response(cx)
}

fn issue_tokens(
    provider: &Provider,
    identity: &Identity,
    session_id: &str,
    nonce: Option<&str>,
) -> Result<Value> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let access_token = sign(
        provider,
        json!({
            "iss": provider.issuer,
            "sub": identity.subject,
            "aud": provider.client_id,
            "azp": provider.client_id,
            "sid": session_id,
            "iat": now,
            "exp": now + provider.access_lifetime
        }),
        "JWT",
    )?;
    let refresh_token = random_token();
    let refresh_expires_in = 7 * 24 * 60 * 60;
    provider
        .refresh_tokens
        .lock()
        .expect("dev OIDC refresh lock poisoned")
        .insert(
            refresh_token.clone(),
            PendingRefresh {
                identity: identity.clone(),
                session_id: session_id.to_owned(),
                expires_at: Instant::now() + Duration::from_secs(refresh_expires_in),
            },
        );
    let id_token = nonce.map(|nonce| {
        sign(
            provider,
            json!({
                "iss": provider.issuer,
                "sub": identity.subject,
                "aud": provider.client_id,
                "email": identity.email,
                "sid": session_id,
                "nonce": nonce,
                "iat": now,
                "exp": now + 3600
            }),
            "JWT",
        )
    });

    Ok(json!({
        "access_token": access_token,
        "refresh_token": refresh_token,
        "token_type": "Bearer",
        "expires_in": provider.access_lifetime,
        "refresh_expires_in": refresh_expires_in,
        "id_token": id_token.transpose()?
    }))
}

fn sign(provider: &Provider, claims: Value, token_type: &str) -> Result<String> {
    let header = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&json!({
        "alg": "RS256",
        "typ": token_type,
        "kid": "dev"
    }))?);
    let claims = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims)?);
    let signing_input = format!("{header}.{claims}");
    let digest = Sha256::digest(signing_input.as_bytes());
    let signature = provider.key.sign(Pkcs1v15Sign::new::<Sha256>(), &digest)?;
    Ok(format!(
        "{signing_input}.{}",
        URL_SAFE_NO_PAD.encode(signature)
    ))
}

fn authorize_url(provider: &Provider, query: &AuthorizeQuery, subject: &str) -> Result<String> {
    let mut url = Url::parse(&format!("{}/authorize", provider.issuer))?;
    url.query_pairs_mut()
        .append_pair("client_id", &query.client_id)
        .append_pair("redirect_uri", &query.redirect_uri)
        .append_pair("response_type", &query.response_type)
        .append_pair("scope", &query.scope)
        .append_pair("state", &query.state)
        .append_pair("nonce", &query.nonce)
        .append_pair("code_challenge", &query.code_challenge)
        .append_pair("code_challenge_method", &query.code_challenge_method)
        .append_pair("sub", subject);
    Ok(url.into())
}

fn random_token() -> String {
    let mut bytes = [0; 32];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn provider(cx: &Cx) -> &Provider {
    app_context(cx)
}

fn oauth_error(code: &str, cx: &Cx) -> Result<Response> {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": code }))).into_response(cx)
}

fn redirect(location: &str, cx: &Cx) -> Result<Response> {
    (
        StatusCode::FOUND,
        [(topcoat::router::header::LOCATION, location)],
        (),
    )
        .into_response(cx)
}

fn required(name: &str) -> anyhow::Result<String> {
    std::env::var(name).map_err(|_| anyhow::anyhow!("{name} must be set"))
}

fn optional_number(name: &str, fallback: u64) -> anyhow::Result<u64> {
    match std::env::var(name) {
        Ok(value) => value.parse().map_err(Into::into),
        Err(_) => Ok(fallback),
    }
}
