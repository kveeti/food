use std::collections::HashMap;

use anyhow::{Context as _, Result, anyhow, bail};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use rand::{RngCore, rngs::OsRng};
use reqwest::Client;
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;
use sha2::{Digest, Sha256};
use url::Url;

use crate::{auth::jwks::Jwks, config::Config};

const LOGOUT_EVENT: &str = "http://schemas.openid.net/event/backchannel-logout";
const LOGOUT_TOKEN_MAX_AGE_SECONDS: i64 = 5 * 60;
const LOGOUT_TOKEN_CLOCK_SKEW_SECONDS: i64 = 60;

pub struct Oidc {
    pub app_url: Url,
    pub issuer: String,
    pub client_id: String,
    client_secret: String,
    audience: Option<String>,
    authorization_endpoint: Url,
    token_endpoint: Url,
    jwks: Jwks,
    http: Client,
}

pub struct Authorization {
    pub url: Url,
    pub state: String,
    pub nonce: String,
    pub code_verifier: String,
}

pub struct ProviderTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub refresh_expires_at: DateTime<Utc>,
    pub access: AccessClaims,
}

pub struct Login {
    pub issuer: String,
    pub subject: String,
    pub email: Option<String>,
    pub oidc_session_id: String,
    pub tokens: ProviderTokens,
}

pub struct AccessClaims {
    pub issuer: String,
    pub subject: String,
    pub expires_at: DateTime<Utc>,
}

pub struct Logout {
    pub issuer: String,
    pub session_id: String,
}

pub enum RefreshError {
    InvalidGrant,
    Other(anyhow::Error),
}

pub enum VerifyError {
    Invalid(anyhow::Error),
    Unavailable(anyhow::Error),
}

impl VerifyError {
    fn invalid(error: impl Into<anyhow::Error>) -> Self {
        Self::Invalid(error.into())
    }

    fn unavailable(error: impl Into<anyhow::Error>) -> Self {
        Self::Unavailable(error.into())
    }

    fn into_error(self) -> anyhow::Error {
        match self {
            Self::Invalid(error) | Self::Unavailable(error) => error,
        }
    }
}

#[derive(Deserialize)]
struct Discovery {
    issuer: String,
    authorization_endpoint: String,
    token_endpoint: String,
    jwks_uri: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    refresh_expires_in: i64,
    id_token: Option<String>,
}

#[derive(Deserialize)]
struct OAuthError {
    error: String,
    error_description: Option<String>,
}

#[derive(Deserialize)]
struct Claims {
    iss: String,
    sub: String,
    aud: Audience,
    exp: i64,
    nonce: Option<String>,
    email: Option<String>,
    sid: Option<String>,
    azp: Option<String>,
}

#[derive(Deserialize)]
struct LogoutClaims {
    iss: String,
    aud: Audience,
    iat: i64,
    jti: String,
    sid: Option<String>,
    nonce: Option<String>,
    events: HashMap<String, Value>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Audience {
    One(String),
    Many(Vec<String>),
}

impl Audience {
    fn contains(&self, expected: &str) -> bool {
        match self {
            Self::One(value) => value == expected,
            Self::Many(values) => values.iter().any(|value| value == expected),
        }
    }
}

impl Oidc {
    #[tracing::instrument(name = "oidc::discover", level = "info", skip_all)]
    pub async fn discover(config: &Config) -> Result<Self> {
        let http = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        let discovery_url = Url::parse(&format!(
            "{}/.well-known/openid-configuration",
            config.oidc_issuer.as_str().trim_end_matches('/')
        ))?;
        let metadata: Discovery = http
            .get(discovery_url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        if metadata.issuer.trim_end_matches('/')
            != config.oidc_issuer.as_str().trim_end_matches('/')
        {
            bail!("OIDC discovery returned a different issuer");
        }
        let authorization_endpoint = validate_endpoint_url(
            &config.oidc_issuer,
            &metadata.authorization_endpoint,
            "authorization",
        )?;
        let token_endpoint =
            validate_endpoint_url(&config.oidc_issuer, &metadata.token_endpoint, "token")?;
        let jwks_url = validate_endpoint_url(&config.oidc_issuer, &metadata.jwks_uri, "JWKS")?;
        let jwks = Jwks::new(http.clone(), jwks_url).await?;

        Ok(Self {
            app_url: config.app_url.clone(),
            issuer: metadata.issuer,
            client_id: config.oidc_client_id.clone(),
            client_secret: config.oidc_client_secret.clone(),
            audience: config.oidc_audience.clone(),
            authorization_endpoint,
            token_endpoint,
            jwks,
            http,
        })
    }

    #[tracing::instrument(name = "oidc::authorize", level = "debug", skip_all)]
    pub fn authorize(&self) -> Result<Authorization> {
        let state = random_token();
        let nonce = random_token();
        let code_verifier = random_token();
        let code_challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(code_verifier.as_bytes()));
        let mut url = self.authorization_endpoint.clone();
        url.query_pairs_mut()
            .append_pair("client_id", &self.client_id)
            .append_pair("redirect_uri", &format!("{}auth/callback", self.app_url))
            .append_pair("response_type", "code")
            .append_pair("scope", "openid email")
            .append_pair("code_challenge", &code_challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("state", &state)
            .append_pair("nonce", &nonce);

        Ok(Authorization {
            url,
            state,
            nonce,
            code_verifier,
        })
    }

    #[tracing::instrument(name = "oidc::exchange", level = "info", skip_all)]
    pub async fn exchange(
        &self,
        callback_url: &Url,
        code: &str,
        nonce: &str,
        code_verifier: &str,
    ) -> Result<Login> {
        let response = self
            .http
            .post(self.token_endpoint.clone())
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", code),
                ("redirect_uri", callback_url.as_str()),
                ("client_id", &self.client_id),
                ("client_secret", &self.client_secret),
                ("code_verifier", code_verifier),
            ])
            .send()
            .await?;
        let token = read_token_response(response).await?;
        let id_token = token
            .id_token
            .as_deref()
            .context("OIDC provider returned no ID token")?;
        let identity: Claims = self
            .verify_jwt(id_token, true)
            .await
            .map_err(VerifyError::into_error)?;
        if identity.nonce.as_deref() != Some(nonce) || !identity.aud.contains(&self.client_id) {
            bail!("OIDC ID token has the wrong nonce or audience");
        }
        let session_id = identity.sid.context("OIDC ID token has no session ID")?;
        let tokens = self.provider_tokens(token, None).await?;
        if identity.iss != tokens.access.issuer || identity.sub != tokens.access.subject {
            bail!("OIDC tokens identify different users");
        }

        Ok(Login {
            issuer: identity.iss,
            subject: identity.sub,
            email: identity.email,
            oidc_session_id: session_id,
            tokens,
        })
    }

    #[tracing::instrument(name = "oidc::refresh", level = "info", skip_all)]
    pub async fn refresh(&self, refresh_token: &str) -> Result<ProviderTokens, RefreshError> {
        let response = self
            .http
            .post(self.token_endpoint.clone())
            .form(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh_token),
                ("client_id", &self.client_id),
                ("client_secret", &self.client_secret),
            ])
            .send()
            .await
            .map_err(|error| RefreshError::Other(error.into()))?;
        let token = read_token_response(response).await.map_err(|error| {
            if error
                .downcast_ref::<OAuthResponseError>()
                .is_some_and(|error| error.code == "invalid_grant")
            {
                RefreshError::InvalidGrant
            } else {
                RefreshError::Other(error)
            }
        })?;

        self.provider_tokens(token, Some(refresh_token))
            .await
            .map_err(RefreshError::Other)
    }

    #[tracing::instrument(name = "oidc::verify_access", level = "debug", skip_all)]
    pub async fn verify_access(
        &self,
        token: &str,
    ) -> std::result::Result<AccessClaims, VerifyError> {
        let claims: Claims = self.verify_jwt(token, false).await?;
        let expected = self.audience.as_deref().unwrap_or(&self.client_id);
        if !claims.aud.contains(expected)
            && (self.audience.is_some() || claims.azp.as_deref() != Some(&self.client_id))
        {
            return Err(VerifyError::invalid(anyhow!(
                "OIDC access token has the wrong audience"
            )));
        }
        let expires_at = DateTime::from_timestamp(claims.exp, 0).ok_or_else(|| {
            VerifyError::invalid(anyhow!("OIDC access token has an invalid expiry"))
        })?;

        Ok(AccessClaims {
            issuer: claims.iss,
            subject: claims.sub,
            expires_at,
        })
    }

    #[tracing::instrument(name = "oidc::verify_logout", level = "debug", skip_all)]
    pub async fn verify_logout(&self, token: &str) -> Result<Logout> {
        let claims: LogoutClaims = self
            .verify_jwt(token, false)
            .await
            .map_err(VerifyError::into_error)?;
        if !claims.aud.contains(&self.client_id)
            || !valid_logout_iat(claims.iat, Utc::now().timestamp())
            || claims.jti.is_empty()
            || claims.nonce.is_some()
            || !claims.events.contains_key(LOGOUT_EVENT)
        {
            bail!("invalid OIDC back-channel logout token");
        }

        Ok(Logout {
            issuer: claims.iss,
            session_id: claims.sid.context("OIDC logout token has no session ID")?,
        })
    }

    #[tracing::instrument(name = "oidc::provider_tokens", level = "debug", skip_all)]
    async fn provider_tokens(
        &self,
        token: TokenResponse,
        previous_refresh_token: Option<&str>,
    ) -> Result<ProviderTokens> {
        if token.refresh_expires_in <= 0 {
            bail!("OIDC provider returned no refresh token lifetime");
        }
        let refresh_token = token
            .refresh_token
            .or_else(|| previous_refresh_token.map(str::to_owned))
            .context("OIDC provider returned no refresh token")?;
        let access = self
            .verify_access(&token.access_token)
            .await
            .map_err(VerifyError::into_error)?;

        Ok(ProviderTokens {
            access_token: token.access_token,
            refresh_token,
            refresh_expires_at: Utc::now() + chrono::Duration::seconds(token.refresh_expires_in),
            access,
        })
    }

    #[tracing::instrument(name = "oidc::verify_jwt", level = "debug", skip_all)]
    async fn verify_jwt<T: DeserializeOwned>(
        &self,
        token: &str,
        require_exp: bool,
    ) -> std::result::Result<T, VerifyError> {
        let header = decode_header(token).map_err(VerifyError::invalid)?;
        if header.alg != Algorithm::RS256 {
            return Err(VerifyError::invalid(anyhow!(
                "OIDC token uses an unsupported signing algorithm"
            )));
        }
        let kid = header
            .kid
            .ok_or_else(|| VerifyError::invalid(anyhow!("OIDC token has no key ID")))?;
        let key = self
            .jwks
            .key(&kid)
            .await
            .map_err(VerifyError::unavailable)?
            .ok_or_else(|| VerifyError::unavailable(anyhow!("OIDC signing key is not cached")))?;
        let key = DecodingKey::from_rsa_components(&key.n, &key.e).map_err(VerifyError::invalid)?;
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&self.issuer]);
        validation.validate_aud = false;
        if !require_exp {
            validation.required_spec_claims.remove("exp");
            validation.validate_exp = false;
        }

        Ok(decode::<T>(token, &key, &validation)
            .map_err(VerifyError::invalid)?
            .claims)
    }
}

#[derive(Debug)]
struct OAuthResponseError {
    code: String,
    description: Option<String>,
}

impl std::fmt::Display for OAuthResponseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "OIDC token endpoint returned {}", self.code)?;
        if let Some(description) = &self.description {
            write!(formatter, ": {description}")?;
        }
        Ok(())
    }
}

impl std::error::Error for OAuthResponseError {}

#[tracing::instrument(name = "oidc::read_token_response", level = "debug", skip_all)]
async fn read_token_response(response: reqwest::Response) -> Result<TokenResponse> {
    if response.status().is_success() {
        return Ok(response.json().await?);
    }

    let error: OAuthError = response.json().await?;
    Err(OAuthResponseError {
        code: error.error,
        description: error.error_description,
    }
    .into())
}

fn valid_logout_iat(iat: i64, now: i64) -> bool {
    iat >= now.saturating_sub(LOGOUT_TOKEN_MAX_AGE_SECONDS)
        && iat <= now.saturating_add(LOGOUT_TOKEN_CLOCK_SKEW_SECONDS)
}

fn validate_endpoint_url(issuer: &Url, value: &str, name: &str) -> Result<Url> {
    let url = Url::parse(value)
        .with_context(|| format!("OIDC discovery returned an invalid {name} URL"))?;
    if !matches!(url.scheme(), "http" | "https") {
        bail!("OIDC {name} URL must use HTTP or HTTPS");
    }
    if issuer.scheme() == "https" && url.scheme() != "https" {
        bail!("OIDC {name} URL must use HTTPS");
    }
    if !url.username().is_empty() || url.password().is_some() {
        bail!("OIDC {name} URL must not contain credentials");
    }
    Ok(url)
}

pub fn random_token() -> String {
    let mut bytes = [0; 32];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::{
        LOGOUT_TOKEN_CLOCK_SKEW_SECONDS, LOGOUT_TOKEN_MAX_AGE_SECONDS, valid_logout_iat,
        validate_endpoint_url,
    };
    use url::Url;

    #[test]
    fn production_oidc_endpoints_must_use_https() {
        let issuer = Url::parse("https://id.example.com").unwrap();

        assert!(validate_endpoint_url(&issuer, "https://id.example.com/endpoint", "token").is_ok());
        assert!(validate_endpoint_url(&issuer, "http://id.example.com/endpoint", "token").is_err());
    }

    #[test]
    fn development_oidc_endpoints_may_use_http() {
        let issuer = Url::parse("http://127.0.0.1:8001").unwrap();

        assert!(validate_endpoint_url(&issuer, "http://127.0.0.1:8001/endpoint", "token").is_ok());
    }

    #[test]
    fn logout_tokens_must_be_recent() {
        let now = 1_000_000;

        assert!(valid_logout_iat(now - LOGOUT_TOKEN_MAX_AGE_SECONDS, now));
        assert!(valid_logout_iat(now + LOGOUT_TOKEN_CLOCK_SKEW_SECONDS, now));
        assert!(!valid_logout_iat(
            now - LOGOUT_TOKEN_MAX_AGE_SECONDS - 1,
            now
        ));
        assert!(!valid_logout_iat(
            now + LOGOUT_TOKEN_CLOCK_SKEW_SECONDS + 1,
            now
        ));
    }
}
