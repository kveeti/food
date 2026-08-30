mod cipher;
mod cookies;
mod jwks;
mod oidc;
pub(crate) mod routes;

use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use self::{
    cookies::{SESSION_COOKIE, remove_cookie, set_session_cookie},
    oidc::{AccessClaims, Oidc, RefreshError, VerifyError},
};
use crate::{
    auth::cipher::TokenCipher,
    config::Config,
    data::{Data, Session, SessionChange},
};
use anyhow::{Result as AnyResult, anyhow};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use rand::{RngCore, rngs::OsRng};
use sha2::{Digest, Sha256};
use topcoat::{
    Result,
    context::{Cx, app_context},
    cookie::{Cookies as _, cookies},
    router::{error::service_unavailable, request::uri},
};
use uuid::Uuid;

const EARLY_REFRESH_SECONDS: i64 = 60;
const REFRESH_RETRY_SECONDS: i64 = 10;
const MAX_ONGOING_EARLY_REFRESHES: usize = 5;
const AUTH_RETRY_AFTER_SECONDS: u64 = 60;

pub struct Auth {
    oidc: Oidc,
    cipher: TokenCipher,
    secure_cookies: bool,
    ongoing_early_refreshes: Mutex<HashSet<Vec<u8>>>,
}

pub struct User {
    id: Uuid,
    pub email: Option<String>,
}

struct Authenticated {
    user: User,
    expires_at: DateTime<Utc>,
}

enum Authentication {
    Authenticated(Authenticated),
    Unauthorized,
    Unavailable,
}

enum AccessStatus {
    Valid(AccessClaims),
    Expired,
    Invalid(anyhow::Error),
    Unavailable(anyhow::Error),
}

#[derive(Clone, Copy)]
enum RefreshAttempt {
    Required,
    Early,
}

impl Auth {
    #[tracing::instrument(name = "auth::new", level = "info", skip_all)]
    pub async fn new(config: &Config) -> AnyResult<Self> {
        let mut oidc = None;
        let mut last_error = None;
        for attempt in 1..=10 {
            match Oidc::discover(config).await {
                Ok(discovered) => {
                    oidc = Some(discovered);
                    break;
                }
                Err(error) => {
                    tracing::warn!(attempt, ?error, "OIDC discovery failed");
                    last_error = Some(error);
                    if attempt < 10 {
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                }
            }
        }
        let oidc =
            oidc.ok_or_else(|| last_error.unwrap_or_else(|| anyhow!("OIDC discovery failed")))?;

        Ok(Self {
            oidc,
            cipher: TokenCipher::new(&config.session_encryption_key),
            secure_cookies: config.app_url.scheme() == "https",
            ongoing_early_refreshes: Mutex::new(HashSet::new()),
        })
    }
}

#[tracing::instrument(name = "auth::require_user", level = "debug", skip_all)]
pub async fn require_user(cx: &Cx) -> Result<User> {
    let return_to = uri(cx).path_and_query().map_or("/", |value| value.as_str());
    let sign_in_url = format!("/sign-in?{}", return_query(return_to));
    let Some(token) = cookies(cx).get(SESSION_COOKIE) else {
        return Err(topcoat::router::error::redirect(&sign_in_url).into());
    };
    let token = token.value().to_owned();

    match authenticate(Arc::clone(auth(cx)), data(cx).to_owned(), hash(&token)).await? {
        Authentication::Authenticated(result) => {
            set_session_cookie(cx, &token, result.expires_at, auth(cx).secure_cookies);
            crate::http::record_user(cx, result.user.id);
            Ok(result.user)
        }
        Authentication::Unauthorized => {
            remove_cookie(cx, SESSION_COOKIE);
            Err(topcoat::router::error::redirect(&sign_in_url).into())
        }
        Authentication::Unavailable => Err(service_unavailable(AUTH_RETRY_AFTER_SECONDS).into()),
    }
}

#[tracing::instrument(name = "auth::authenticate", level = "debug", skip_all)]
async fn authenticate(
    auth: Arc<Auth>,
    data: Data,
    token_hash: Vec<u8>,
) -> AnyResult<Authentication> {
    let Some(session) = data.session(&token_hash).await? else {
        return Ok(Authentication::Unauthorized);
    };
    let claims = match check_access(&auth, &session).await {
        AccessStatus::Valid(claims) => claims,
        AccessStatus::Unavailable(error) => {
            tracing::warn!(?error, "OIDC signing keys unavailable");
            return Ok(Authentication::Unavailable);
        }
        AccessStatus::Expired | AccessStatus::Invalid(_) => {
            return refresh_session(&auth, &data, &token_hash, RefreshAttempt::Required).await;
        }
    };
    if claims.expires_at <= Utc::now() + ChronoDuration::seconds(EARLY_REFRESH_SECONDS)
        && session
            .refresh_retry_after
            .is_none_or(|retry| retry <= Utc::now())
    {
        start_early_refresh(auth, data, token_hash);
    }

    Ok(Authentication::Authenticated(authenticated(&session)))
}

fn start_early_refresh(auth: Arc<Auth>, data: Data, token_hash: Vec<u8>) {
    {
        let mut ongoing = match auth.ongoing_early_refreshes.lock() {
            Ok(ongoing) => ongoing,
            Err(poisoned) => {
                tracing::error!("early refresh lock was poisoned");
                poisoned.into_inner()
            }
        };
        if ongoing.len() >= MAX_ONGOING_EARLY_REFRESHES || !ongoing.insert(token_hash.to_owned()) {
            return;
        }
    }

    tokio::spawn(async move {
        if let Err(error) = refresh_session(&auth, &data, &token_hash, RefreshAttempt::Early).await
        {
            tracing::warn!(?error, "early OIDC refresh failed");
        }
        let mut ongoing = match auth.ongoing_early_refreshes.lock() {
            Ok(ongoing) => ongoing,
            Err(poisoned) => {
                tracing::error!("early refresh lock was poisoned");
                poisoned.into_inner()
            }
        };
        ongoing.remove(&token_hash);
    });
}

#[tracing::instrument(name = "auth::refresh_session", level = "info", skip_all)]
async fn refresh_session(
    auth: &Auth,
    data: &Data,
    token_hash: &[u8],
    attempt: RefreshAttempt,
) -> AnyResult<Authentication> {
    let refresh_before_expiry_seconds = match attempt {
        RefreshAttempt::Required => 0,
        RefreshAttempt::Early => EARLY_REFRESH_SECONDS,
    };
    let update = match attempt {
        RefreshAttempt::Required => data.update_session(token_hash),
        RefreshAttempt::Early => data.try_update_session(token_hash),
    };
    let result = update
        .run(move |session| async move {
            match check_access(auth, &session).await {
                AccessStatus::Unavailable(error) => {
                    tracing::warn!(?error, "OIDC signing keys unavailable");
                    return Ok((Authentication::Unavailable, SessionChange::Keep));
                }
                AccessStatus::Invalid(error) => {
                    tracing::error!(?error, "invalid stored OIDC access token; deleting session");
                    return Ok((Authentication::Unauthorized, SessionChange::Delete));
                }
                AccessStatus::Valid(claims)
                    if claims.expires_at
                        > Utc::now() + ChronoDuration::seconds(refresh_before_expiry_seconds)
                        || session
                            .refresh_retry_after
                            .is_some_and(|retry| retry > Utc::now()) =>
                {
                    return Ok((
                        Authentication::Authenticated(authenticated(&session)),
                        SessionChange::Keep,
                    ));
                }
                _ => {}
            }

            if session
                .refresh_retry_after
                .is_some_and(|retry| retry > Utc::now())
            {
                return Ok((Authentication::Unavailable, SessionChange::Keep));
            }

            let refresh_token = match auth
                .cipher
                .decrypt(&session.refresh_token, &format!("{}:refresh", session.id))
            {
                Ok(token) => token,
                Err(error) => {
                    tracing::error!(
                        ?error,
                        "stored OIDC refresh token decryption failed; deleting session"
                    );
                    return Ok((Authentication::Unauthorized, SessionChange::Delete));
                }
            };

            let tokens = match auth.oidc.refresh(&refresh_token).await {
                Ok(tokens) => tokens,
                Err(RefreshError::InvalidGrant) => {
                    tracing::warn!("OIDC refresh token rejected");
                    return Ok((Authentication::Unauthorized, SessionChange::Delete));
                }
                Err(RefreshError::Other(error)) => {
                    tracing::warn!(?error, "OIDC session refresh failed");
                    return Ok((
                        Authentication::Unavailable,
                        SessionChange::RetryAfter(
                            Utc::now() + ChronoDuration::seconds(REFRESH_RETRY_SECONDS),
                        ),
                    ));
                }
            };
            if tokens.access.issuer != session.issuer || tokens.access.subject != session.subject {
                return Ok((Authentication::Unauthorized, SessionChange::Delete));
            }

            let id = session.id;
            let result = Authenticated {
                user: User {
                    id: session.user_id,
                    email: session.email,
                },
                expires_at: tokens.refresh_expires_at,
            };
            Ok((
                Authentication::Authenticated(result),
                SessionChange::SaveTokens {
                    access_token: auth
                        .cipher
                        .encrypt(&tokens.access_token, &format!("{id}:access"))?,
                    refresh_token: auth
                        .cipher
                        .encrypt(&tokens.refresh_token, &format!("{id}:refresh"))?,
                    refresh_expires_at: tokens.refresh_expires_at,
                },
            ))
        })
        .await?;

    match result {
        Some(result) => Ok(result),
        None => Ok(Authentication::Unauthorized),
    }
}

#[tracing::instrument(name = "auth::check_access", level = "debug", skip_all)]
async fn check_access(auth: &Auth, session: &Session) -> AccessStatus {
    let token = match auth
        .cipher
        .decrypt(&session.access_token, &format!("{}:access", session.id))
    {
        Ok(token) => token,
        Err(error) => return AccessStatus::Invalid(error),
    };
    let claims = match auth.oidc.verify_access(&token).await {
        Ok(claims) => claims,
        Err(VerifyError::Invalid(error)) => return AccessStatus::Invalid(error),
        Err(VerifyError::Unavailable(error)) => return AccessStatus::Unavailable(error),
    };
    if claims.issuer != session.issuer || claims.subject != session.subject {
        return AccessStatus::Invalid(anyhow!(
            "OIDC access token identity does not match its session"
        ));
    }
    if claims.expires_at <= Utc::now() {
        AccessStatus::Expired
    } else {
        AccessStatus::Valid(claims)
    }
}

fn authenticated(session: &Session) -> Authenticated {
    Authenticated {
        user: User {
            id: session.user_id,
            email: session.email.to_owned(),
        },
        expires_at: session.refresh_expires_at,
    }
}

fn auth(cx: &Cx) -> &Arc<Auth> {
    app_context(cx)
}

fn data(cx: &Cx) -> &Data {
    app_context(cx)
}

fn return_query(value: &str) -> String {
    url::form_urlencoded::Serializer::new(String::new())
        .append_pair("return_to", value)
        .finish()
}

fn random_cookie_token() -> String {
    let mut bytes = [0; 32];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn hash(value: &str) -> Vec<u8> {
    Sha256::digest(value.as_bytes()).to_vec()
}
