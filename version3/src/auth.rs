use std::{env, time::Duration as StdDuration};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use openidconnect::{
    AccessTokenHash, AuthType, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    EndpointMaybeSet, EndpointNotSet, EndpointSet, IssuerUrl, Nonce, OAuth2TokenResponse,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
    core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata},
    reqwest,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, types::Uuid};
use tokio::sync::OnceCell;
use topcoat::{
    Result,
    context::{Cx, app_context},
    cookie::{Cookie, Cookies, SameSite, cookies, time::Duration},
    router::{
        HeaderValue, IntoResponse, Response, RouterBuilder, StatusCode,
        error::{RouterErrorExt, bad_request, see_other, unauthorized},
        header, headers, query_params, route,
    },
    session::{self, Token, TokenStore, TokenStoreFuture},
};
use url::Url;

const FLOW_COOKIE: &str = "oidc_flow";
const DEV_SESSION_COOKIE: &str = "dev_session";

pub struct DevCookieTokenStore;

impl TokenStore for DevCookieTokenStore {
    fn read<'a>(&'a self, cx: &'a Cx) -> TokenStoreFuture<'a, Option<Token>> {
        Box::pin(async move {
            let Some(cookie) = cookies(cx).get(DEV_SESSION_COOKIE) else {
                return Ok(None);
            };
            Ok(Token::decode(cookie.value()).ok())
        })
    }

    fn write<'a>(
        &'a self,
        cx: &'a Cx,
        token: Token,
        max_age: StdDuration,
    ) -> TokenStoreFuture<'a, ()> {
        Box::pin(async move {
            cookies(cx).add(
                Cookie::build((DEV_SESSION_COOKIE, token.encode()))
                    .path("/")
                    .http_only(true)
                    .secure(false)
                    .same_site(SameSite::Lax)
                    .max_age(Duration::try_from(max_age)?)
                    .build(),
            );
            Ok(())
        })
    }

    fn delete<'a>(&'a self, cx: &'a Cx) -> TokenStoreFuture<'a, ()> {
        Box::pin(async move {
            cookies(cx).remove(Cookie::build((DEV_SESSION_COOKIE, "")).path("/").build());
            Ok(())
        })
    }
}

type OidcClient = CoreClient<
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointMaybeSet,
    EndpointMaybeSet,
>;

pub struct Auth {
    pub is_prod: bool,
    pub app_url: String,
    pub issuer: String,
    pub client_id: String,
    pub client_secret: String,
    admin_claim: String,
    admin_value: String,
    scopes: Vec<String>,
    client: OnceCell<OidcClient>,
    http: reqwest::Client,
}

impl Auth {
    pub fn from_env() -> Self {
        let is_prod = env::var("IS_PROD").as_deref() == Ok("1");
        let port = env::var("PORT").unwrap_or_else(|_| "3000".to_owned());
        let app_url = env::var("APP_URL")
            .unwrap_or_else(|_| format!("http://localhost:{port}"))
            .trim_end_matches('/')
            .to_owned();

        let (issuer, client_id, client_secret) = if is_prod {
            (
                required_env("OIDC_ISSUER"),
                required_env("OIDC_CLIENT_ID"),
                required_env("OIDC_CLIENT_SECRET"),
            )
        } else {
            (
                format!("{app_url}/dev/oidc"),
                "food-dev".to_owned(),
                "food-dev-secret".to_owned(),
            )
        };

        let (admin_claim, admin_value, scopes) = if is_prod {
            (
                required_env("OIDC_ADMIN_CLAIM"),
                required_env("OIDC_ADMIN_VALUE"),
                env::var("OIDC_SCOPES").unwrap_or_else(|_| "email".to_owned()),
            )
        } else {
            (
                "groups".to_owned(),
                "food-admin".to_owned(),
                "email groups".to_owned(),
            )
        };
        let scopes = scopes
            .split([',', ' '])
            .filter(|scope| !scope.is_empty() && *scope != "openid")
            .map(str::to_owned)
            .collect();

        let http = reqwest::ClientBuilder::new()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(StdDuration::from_secs(10))
            .build()
            .expect("OIDC HTTP client must build");

        Self {
            is_prod,
            app_url,
            issuer,
            client_id,
            client_secret,
            admin_claim,
            admin_value,
            scopes,
            client: OnceCell::new(),
            http,
        }
    }

    async fn client(&self) -> Result<&OidcClient> {
        self.client
            .get_or_try_init(|| async {
                let metadata = CoreProviderMetadata::discover_async(
                    IssuerUrl::new(self.issuer.clone())?,
                    &self.http,
                )
                .await
                .inspect_err(|error| eprintln!("OIDC discovery failed: {error}"))?;

                let client = CoreClient::from_provider_metadata(
                    metadata,
                    ClientId::new(self.client_id.clone()),
                    Some(ClientSecret::new(self.client_secret.clone())),
                )
                .set_redirect_uri(RedirectUrl::new(format!("{}/auth/callback", self.app_url))?);

                Ok(if self.is_prod {
                    client
                } else {
                    client.set_auth_type(AuthType::RequestBody)
                })
            })
            .await
    }
}

fn required_env(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("{name} must be set when IS_PROD=1"))
}

pub fn register(builder: RouterBuilder) -> RouterBuilder {
    builder.route(login).route(callback).route(logout)
}

fn auth(cx: &Cx) -> &Auth {
    app_context(cx)
}

fn db(cx: &Cx) -> &PgPool {
    app_context(cx)
}

#[derive(Debug, Serialize, Deserialize)]
struct LoginFlow {
    state: String,
    nonce: String,
    pkce_verifier: String,
}

fn save_flow(cx: &Cx, flow: &LoginFlow) -> Result<()> {
    let value = URL_SAFE_NO_PAD.encode(serde_json::to_vec(flow)?);
    cookies(cx).add(
        Cookie::build((FLOW_COOKIE, value))
            .path("/")
            .http_only(true)
            .secure(auth(cx).is_prod)
            .same_site(SameSite::Lax)
            .max_age(Duration::minutes(10))
            .build(),
    );
    Ok(())
}

fn take_flow(cx: &Cx) -> Result<LoginFlow> {
    let jar = cookies(cx);
    let cookie = jar
        .get(FLOW_COOKIE)
        .ok_or_else(|| bad_request("missing OIDC login cookie"))?;
    let bytes = URL_SAFE_NO_PAD
        .decode(cookie.value())
        .map_err(|_| bad_request("invalid OIDC login cookie"))?;
    let flow =
        serde_json::from_slice(&bytes).map_err(|_| bad_request("invalid OIDC login cookie"))?;

    jar.remove(Cookie::build((FLOW_COOKIE, "")).path("/").build());
    Ok(flow)
}

#[route(GET "/login")]
pub async fn login(cx: &Cx) -> Result<Response> {
    let auth = auth(cx);
    if !auth.is_prod {
        let expected = Url::parse(&auth.app_url)?;
        let expected_host = match expected.port() {
            Some(port) => format!("{}:{port}", expected.host_str().unwrap_or_default()),
            None => expected.host_str().unwrap_or_default().to_owned(),
        };
        let request_host = headers(cx)
            .get(header::HOST)
            .and_then(|host| host.to_str().ok());
        if request_host != Some(expected_host.as_str()) {
            return (
                StatusCode::FOUND,
                [(
                    header::LOCATION,
                    HeaderValue::from_str(&format!("{}/login", auth.app_url))?,
                )],
                (),
            )
                .into_response(cx);
        }
    }

    let client = auth.client().await?;
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let (url, state, nonce) = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scopes(auth.scopes.iter().cloned().map(Scope::new))
        .set_pkce_challenge(pkce_challenge)
        .url();

    save_flow(
        cx,
        &LoginFlow {
            state: state.secret().to_owned(),
            nonce: nonce.secret().to_owned(),
            pkce_verifier: pkce_verifier.secret().to_owned(),
        },
    )?;

    (
        StatusCode::FOUND,
        [(header::LOCATION, HeaderValue::from_str(url.as_str())?)],
        (),
    )
        .into_response(cx)
}

#[topcoat::router::query_params(error = bad_request)]
struct CallbackQuery {
    code: String,
    state: String,
}

#[route(GET "/auth/callback")]
pub async fn callback(cx: &Cx) -> Result<Response> {
    let query = query_params::<CallbackQuery>(cx)?;
    let flow = take_flow(cx)?;
    if !secret_eq(&query.state, &flow.state) {
        return Err(bad_request("OIDC state mismatch").into());
    }

    let client = auth(cx).client().await?;
    let request = client
        .exchange_code(AuthorizationCode::new(query.code.clone()))
        .map_err(|error| {
            eprintln!("OIDC code exchange setup failed: {error}");
            unauthorized()
        })?
        .set_pkce_verifier(PkceCodeVerifier::new(flow.pkce_verifier));
    let token = request
        .request_async(&auth(cx).http)
        .await
        .map_err(|error| {
            eprintln!("OIDC code exchange failed: {error}");
            unauthorized()
        })?;
    let id_token = token.id_token().ok_or_else(unauthorized)?;
    let verifier = client.id_token_verifier();
    let claims = id_token
        .claims(&verifier, &Nonce::new(flow.nonce))
        .map_err(|error| {
            eprintln!("OIDC ID token check failed: {error}");
            unauthorized()
        })?;

    if let Some(expected) = claims.access_token_hash() {
        let actual = AccessTokenHash::from_token(
            token.access_token(),
            id_token.signing_alg()?,
            id_token.signing_key(&verifier)?,
        )?;
        if actual != *expected {
            return Err(unauthorized().into());
        }
    }

    let issuer = claims.issuer().as_str();
    let subject = claims.subject().as_str();
    let email = claims.email().map_or("", |email| email.as_str());
    let is_admin = has_admin_claim(
        &id_token.to_string(),
        &auth(cx).admin_claim,
        &auth(cx).admin_value,
    );
    let mut tx = db(cx).begin().await?;
    let user_id: Uuid = sqlx::query_scalar(
        "INSERT INTO users (issuer, subject, email)
         VALUES ($1, $2, $3)
         ON CONFLICT (issuer, subject)
         DO UPDATE SET email = EXCLUDED.email
         RETURNING id",
    )
    .bind(issuer)
    .bind(subject)
    .bind(email)
    .fetch_one(&mut *tx)
    .await?;

    let new_session = session::start(cx).await?;
    sqlx::query(
        "INSERT INTO sessions (token_hash, user_id, is_admin, expires_at)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(&new_session.token_hash[..])
    .bind(user_id)
    .bind(is_admin)
    .bind(DateTime::<Utc>::from(new_session.expires_at))
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    see_other("/").into_response(cx)
}

#[route(POST "/logout")]
pub async fn logout(cx: &Cx) -> Result<Response> {
    if let Some(hash) = session::stop(cx).await? {
        sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
            .bind(&hash[..])
            .execute(db(cx))
            .await?;
    }

    see_other("/login").into_response(cx)
}

#[derive(Debug)]
pub struct User {
    pub id: Uuid,
    pub timezone: Option<String>,
    pub is_admin: bool,
}

async fn current_user(cx: &Cx) -> Result<Option<User>> {
    let Some(hash) = session::token_hash(cx).await? else {
        return Ok(None);
    };

    let user = sqlx::query_as(
        "SELECT users.id, users.timezone, sessions.is_admin
         FROM sessions
         JOIN users ON users.id = sessions.user_id
         WHERE sessions.token_hash = $1
           AND sessions.expires_at > now()",
    )
    .bind(&hash[..])
    .fetch_optional(db(cx))
    .await?;

    Ok(user.map(|(id, timezone, is_admin)| User {
        id,
        timezone,
        is_admin,
    }))
}

pub async fn require_user(cx: &Cx) -> Result<User> {
    Ok(current_user(cx).await?.ok_or_redirect("/login")?)
}

pub async fn require_admin(cx: &Cx) -> Result<User> {
    let user = require_user(cx).await?;
    if !user.is_admin {
        return Err(unauthorized().into());
    }
    Ok(user)
}

fn has_admin_claim(id_token: &str, claim: &str, expected: &str) -> bool {
    let Some(payload) = id_token.split('.').nth(1) else {
        return false;
    };
    let Ok(payload) = URL_SAFE_NO_PAD.decode(payload) else {
        return false;
    };
    let Ok(claims) = serde_json::from_slice::<serde_json::Value>(&payload) else {
        return false;
    };
    match &claims[claim] {
        serde_json::Value::String(value) => value == expected,
        serde_json::Value::Array(values) => values.iter().any(|value| value == expected),
        _ => false,
    }
}

fn secret_eq(left: &str, right: &str) -> bool {
    Sha256::digest(left.as_bytes()) == Sha256::digest(right.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(claims: serde_json::Value) -> String {
        format!("x.{}.x", URL_SAFE_NO_PAD.encode(claims.to_string()))
    }

    #[test]
    fn admin_claim_accepts_a_matching_string_or_array() {
        assert!(has_admin_claim(
            &token(serde_json::json!({"role": "admin"})),
            "role",
            "admin"
        ));
        assert!(has_admin_claim(
            &token(serde_json::json!({"groups": ["users", "food-admin"]})),
            "groups",
            "food-admin"
        ));
        assert!(!has_admin_claim(
            &token(serde_json::json!({"groups": ["users"]})),
            "groups",
            "food-admin"
        ));
    }
}
