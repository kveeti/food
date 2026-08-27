use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use openidconnect::CsrfToken;
use rsa::{
    RsaPrivateKey, RsaPublicKey, pkcs1v15::Pkcs1v15Sign, rand_core::OsRng, traits::PublicKeyParts,
};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{
        HeaderValue, IntoResponse, Response, RouterBuilder, StatusCode,
        content::{Form, Json},
        error::bad_request,
        header, query_params, route,
    },
    view::view,
};
use url::Url;

use crate::auth::Auth;

pub struct DevOidc {
    issuer: String,
    callback: String,
    client_id: String,
    client_secret: String,
    key: RsaPrivateKey,
    codes: Mutex<HashMap<String, PendingCode>>,
}

struct PendingCode {
    subject: String,
    email: String,
    nonce: String,
    pkce_challenge: String,
    redirect_uri: String,
    created_at: Instant,
}

impl DevOidc {
    pub fn new(auth: &Auth) -> Self {
        Self {
            issuer: auth.issuer.clone(),
            callback: format!("{}/auth/callback", auth.app_url),
            client_id: auth.client_id.clone(),
            client_secret: auth.client_secret.clone(),
            key: RsaPrivateKey::new(&mut OsRng, 2048).expect("dev OIDC key must generate"),
            codes: Mutex::new(HashMap::new()),
        }
    }
}

pub fn register(builder: RouterBuilder) -> RouterBuilder {
    builder
        .route(discovery)
        .route(jwks)
        .route(authorize)
        .route(token)
}

fn provider(cx: &Cx) -> &DevOidc {
    app_context(cx)
}

#[route(GET "/dev/oidc/.well-known/openid-configuration")]
async fn discovery(cx: &Cx) -> Result<Json<serde_json::Value>> {
    let issuer = &provider(cx).issuer;
    Ok(Json(json!({
        "issuer": issuer,
        "authorization_endpoint": format!("{issuer}/authorize"),
        "token_endpoint": format!("{issuer}/token"),
        "jwks_uri": format!("{issuer}/jwks.json"),
        "response_types_supported": ["code"],
        "subject_types_supported": ["public"],
        "id_token_signing_alg_values_supported": ["RS256"],
        "scopes_supported": ["openid", "email", "groups"],
        "token_endpoint_auth_methods_supported": ["client_secret_post"],
        "code_challenge_methods_supported": ["S256"]
    })))
}

#[route(GET "/dev/oidc/jwks.json")]
async fn jwks(cx: &Cx) -> Result<Json<serde_json::Value>> {
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

#[route(GET "/dev/oidc/authorize")]
async fn authorize(cx: &Cx) -> Result<Response> {
    let query = query_params::<AuthorizeQuery>(cx)?;
    let provider = provider(cx);
    if query.client_id != provider.client_id
        || query.redirect_uri != provider.callback
        || query.response_type != "code"
        || query.code_challenge_method != "S256"
        || query.state.is_empty()
        || query.nonce.is_empty()
    {
        return Err(bad_request("invalid dev OIDC request").into());
    }

    if let Some(subject) = query.sub.as_deref().filter(|subject| !subject.is_empty()) {
        let code = CsrfToken::new_random().secret().to_owned();
        let email = format!("{subject}@dev.local");
        let mut codes = provider.codes.lock().expect("dev OIDC code lock poisoned");
        codes.retain(|_, code| code.created_at.elapsed() < Duration::from_secs(600));
        codes.insert(
            code.clone(),
            PendingCode {
                subject: subject.to_owned(),
                email,
                nonce: query.nonce.clone(),
                pkce_challenge: query.code_challenge.clone(),
                redirect_uri: query.redirect_uri.clone(),
                created_at: Instant::now(),
            },
        );
        drop(codes);

        let mut redirect = Url::parse(&query.redirect_uri)?;
        redirect
            .query_pairs_mut()
            .append_pair("code", &code)
            .append_pair("state", &query.state);
        return (
            StatusCode::FOUND,
            [(header::LOCATION, HeaderValue::from_str(redirect.as_str())?)],
            (),
        )
            .into_response(cx);
    }

    let alice = authorize_url(provider, query, "alice")?;
    let bob = authorize_url(provider, query, "bob")?;
    let page = view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <title>"Dev login"</title>
            </head>
            <body style="font: 16px system-ui; max-width: 24rem; margin: 4rem auto; padding: 1rem">
                <h1>"Pick a dev user"</h1>
                <p><a href=(alice)>"alice - alice@dev.local"</a></p>
                <p><a href=(bob)>"bob - bob@dev.local"</a></p>
                <form method="get" action=(format!("{}/authorize", provider.issuer))>
                    <input type="hidden" name="client_id" value=(&query.client_id)>
                    <input type="hidden" name="redirect_uri" value=(&query.redirect_uri)>
                    <input type="hidden" name="response_type" value=(&query.response_type)>
                    <input type="hidden" name="scope" value=(&query.scope)>
                    <input type="hidden" name="state" value=(&query.state)>
                    <input type="hidden" name="nonce" value=(&query.nonce)>
                    <input type="hidden" name="code_challenge" value=(&query.code_challenge)>
                    <input type="hidden" name="code_challenge_method" value=(&query.code_challenge_method)>
                    <input name="sub" required="true" placeholder="new-user-sub" style="padding: .4rem">
                    <button type="submit" style="padding: .4rem .75rem; border: 1px solid #d1d5db; border-radius: .5rem; background: transparent; font-weight: 500; cursor: pointer">"Log in"</button>
                </form>
            </body>
        </html>
    }?;
    page.into_response(cx)
}

fn authorize_url(provider: &DevOidc, query: &AuthorizeQuery, subject: &str) -> Result<String> {
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

#[derive(Deserialize)]
struct TokenForm {
    grant_type: String,
    code: String,
    redirect_uri: String,
    client_id: String,
    client_secret: String,
    code_verifier: String,
}

#[route(POST "/dev/oidc/token")]
async fn token(cx: &Cx, Form(form): Form<TokenForm>) -> Result<Json<serde_json::Value>> {
    let provider = provider(cx);
    if form.grant_type != "authorization_code"
        || form.client_id != provider.client_id
        || form.client_secret != provider.client_secret
    {
        return Err(bad_request("invalid dev OIDC token request").into());
    }

    let pending = provider
        .codes
        .lock()
        .expect("dev OIDC code lock poisoned")
        .remove(&form.code)
        .ok_or_else(|| bad_request("invalid dev OIDC code"))?;
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(form.code_verifier.as_bytes()));
    if pending.created_at.elapsed() >= Duration::from_secs(600)
        || pending.redirect_uri != form.redirect_uri
        || challenge != pending.pkce_challenge
    {
        return Err(bad_request("invalid dev OIDC code").into());
    }

    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let groups = if pending.subject == "alice" {
        vec!["food-admin"]
    } else {
        Vec::new()
    };
    let id_token = sign(
        provider,
        json!({
            "iss": provider.issuer,
            "sub": pending.subject,
            "aud": provider.client_id,
            "email": pending.email,
            "groups": groups,
            "nonce": pending.nonce,
            "iat": now,
            "exp": now + 3600
        }),
    )?;

    Ok(Json(json!({
        "access_token": CsrfToken::new_random().secret(),
        "token_type": "Bearer",
        "expires_in": 3600,
        "id_token": id_token
    })))
}

fn sign(provider: &DevOidc, claims: serde_json::Value) -> Result<String> {
    let header = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&json!({
        "alg": "RS256",
        "typ": "JWT",
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
