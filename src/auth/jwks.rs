use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use rand::{Rng as _, thread_rng};
use reqwest::{
    Client, StatusCode,
    header::{ETAG, HeaderValue, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED},
};
use serde::Deserialize;
use tokio::sync::{Mutex, RwLock};
use url::Url;

const MAX_AGE: Duration = Duration::from_hours(1);
const ATTEMPT_TIMEOUT: Duration = Duration::from_secs(3);
const RETRY_DEADLINE: Duration = Duration::from_secs(8);
const RETRY_DELAYS: [Duration; 2] = [Duration::from_millis(250), Duration::from_secs(2)];
const FAILURE_COOLDOWN: Duration = Duration::from_mins(1);
const MAX_RESPONSE_BYTES: usize = 1024 * 1024;

pub struct Jwks {
    http: Client,
    uri: Url,
    state: RwLock<State>,
    refresh: Mutex<()>,
}

struct State {
    keys: Vec<Key>,
    validated_at: Instant,
    etag: Option<HeaderValue>,
    last_modified: Option<HeaderValue>,
    retry_after: Option<Instant>,
}

#[derive(Clone, Deserialize)]
pub struct Key {
    pub kid: String,
    pub n: String,
    pub e: String,
}

#[derive(Deserialize)]
struct Document {
    keys: Vec<Key>,
}

enum Fetch {
    Modified {
        keys: Vec<Key>,
        etag: Option<HeaderValue>,
        last_modified: Option<HeaderValue>,
    },
    NotModified {
        etag: Option<HeaderValue>,
        last_modified: Option<HeaderValue>,
    },
}

struct FetchError {
    error: anyhow::Error,
    retryable: bool,
}

impl Jwks {
    #[tracing::instrument(
        name = "jwks::new",
        level = "info",
        skip_all,
        fields(max_age_seconds = MAX_AGE.as_secs())
    )]
    pub async fn new(http: Client, uri: Url) -> Result<Self> {
        let Fetch::Modified {
            keys,
            etag,
            last_modified,
        } = fetch(&http, &uri, None, None)
            .await
            .map_err(|failure| failure.error)?
        else {
            return Err(anyhow!(
                "JWKS endpoint returned 304 without a cached response"
            ));
        };

        Ok(Self {
            http,
            uri,
            state: RwLock::new(State {
                keys,
                validated_at: Instant::now(),
                etag,
                last_modified,
                retry_after: None,
            }),
            refresh: Mutex::new(()),
        })
    }

    #[tracing::instrument(name = "jwks::key", level = "debug", skip_all)]
    pub async fn key(&self, kid: &str) -> Result<Option<Key>> {
        {
            let state = self.state.read().await;
            if state.validated_at.elapsed() < MAX_AGE {
                return Ok(find_key(&state.keys, kid));
            }
            if state
                .retry_after
                .is_some_and(|retry| retry > Instant::now())
            {
                return Err(anyhow!("JWKS refresh is waiting to retry"));
            }
        }

        let _refresh = self.refresh.lock().await;
        {
            let state = self.state.read().await;
            if state.validated_at.elapsed() < MAX_AGE {
                return Ok(find_key(&state.keys, kid));
            }
            if state
                .retry_after
                .is_some_and(|retry| retry > Instant::now())
            {
                return Err(anyhow!("JWKS refresh is waiting to retry"));
            }
        }

        self.revalidate().await?;
        let state = self.state.read().await;
        Ok(find_key(&state.keys, kid))
    }

    #[tracing::instrument(name = "jwks::revalidate", level = "info", skip_all)]
    async fn revalidate(&self) -> Result<()> {
        let (etag, last_modified) = {
            let state = self.state.read().await;
            (state.etag.clone(), state.last_modified.clone())
        };

        let mut attempt = 0;
        let fetched = tokio::time::timeout(RETRY_DEADLINE, async {
            loop {
                match fetch(&self.http, &self.uri, etag.as_ref(), last_modified.as_ref()).await {
                    Ok(fetched) => break Ok(fetched),
                    Err(failure) if failure.retryable && attempt < RETRY_DELAYS.len() => {
                        let delay = jitter(RETRY_DELAYS[attempt]);
                        attempt += 1;
                        tracing::warn!(
                            attempt,
                            ?delay,
                            error = ?failure.error,
                            "JWKS refresh failed; retrying"
                        );
                        tokio::time::sleep(delay).await;
                    }
                    Err(failure) => break Err(failure.error),
                }
            }
        })
        .await
        .unwrap_or_else(|_| Err(anyhow!("JWKS refresh exceeded its retry deadline")));

        let fetched = match fetched {
            Ok(fetched) => fetched,
            Err(error) => {
                self.state.write().await.retry_after = Some(Instant::now() + FAILURE_COOLDOWN);
                return Err(error);
            }
        };

        let mut state = self.state.write().await;
        match fetched {
            Fetch::Modified {
                keys,
                etag,
                last_modified,
            } => {
                state.keys = keys;
                state.etag = etag;
                state.last_modified = last_modified;
            }
            Fetch::NotModified {
                etag,
                last_modified,
            } => {
                if etag.is_some() {
                    state.etag = etag;
                }
                if last_modified.is_some() {
                    state.last_modified = last_modified;
                }
            }
        }
        state.validated_at = Instant::now();
        state.retry_after = None;
        Ok(())
    }
}

fn find_key(keys: &[Key], kid: &str) -> Option<Key> {
    keys.iter().find(|key| key.kid == kid).cloned()
}

fn jitter(max: Duration) -> Duration {
    Duration::from_millis(thread_rng().gen_range(0..=max.as_millis() as u64))
}

async fn fetch(
    http: &Client,
    uri: &Url,
    etag: Option<&HeaderValue>,
    last_modified: Option<&HeaderValue>,
) -> Result<Fetch, FetchError> {
    let mut request = http.get(uri.clone()).timeout(ATTEMPT_TIMEOUT);
    if let Some(etag) = etag {
        request = request.header(IF_NONE_MATCH, etag);
    }
    if let Some(last_modified) = last_modified {
        request = request.header(IF_MODIFIED_SINCE, last_modified);
    }

    let response = request.send().await.map_err(|error| FetchError {
        error: error.into(),
        retryable: true,
    })?;
    let status = response.status();
    let etag = response.headers().get(ETAG).cloned();
    let last_modified = response.headers().get(LAST_MODIFIED).cloned();

    if status == StatusCode::NOT_MODIFIED {
        return Ok(Fetch::NotModified {
            etag,
            last_modified,
        });
    }
    if !status.is_success() {
        return Err(FetchError {
            error: anyhow!("JWKS endpoint returned HTTP {status}"),
            retryable: status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error(),
        });
    }

    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
    {
        return Err(FetchError {
            error: anyhow!("JWKS response exceeds {MAX_RESPONSE_BYTES} bytes"),
            retryable: false,
        });
    }

    let mut response = response;
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|error| FetchError {
        error: error.into(),
        retryable: true,
    })? {
        if body.len() + chunk.len() > MAX_RESPONSE_BYTES {
            return Err(FetchError {
                error: anyhow!("JWKS response exceeds {MAX_RESPONSE_BYTES} bytes"),
                retryable: false,
            });
        }
        body.extend_from_slice(&chunk);
    }
    let document = serde_json::from_slice::<Document>(&body).map_err(|error| FetchError {
        error: error.into(),
        retryable: true,
    })?;
    if document.keys.is_empty() {
        return Err(FetchError {
            error: anyhow!("JWKS endpoint returned no keys"),
            retryable: true,
        });
    }

    Ok(Fetch::Modified {
        keys: document.keys,
        etag,
        last_modified,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering},
    };

    use topcoat::{
        Result as TopcoatResult,
        context::{Cx, app_context},
        router::{Body, Router, StatusCode, request::headers, response::Response, route},
    };

    use super::*;

    const JWKS: &str = r#"{"keys":[{"kid":"one","n":"AQAB","e":"AQAB"}]}"#;
    const LAST_CHANGED: &str = "Wed, 21 Oct 2015 07:28:00 GMT";
    const OK: u8 = 0;
    const NOT_MODIFIED: u8 = 1;
    const UNAVAILABLE: u8 = 2;

    struct ServerState {
        mode: AtomicU8,
        requests: AtomicUsize,
        saw_etag: AtomicBool,
        saw_last_modified: AtomicBool,
        body: Mutex<String>,
    }

    struct Server {
        state: Arc<ServerState>,
        url: Url,
        task: tokio::task::JoinHandle<()>,
    }

    impl Drop for Server {
        fn drop(&mut self) {
            self.task.abort();
        }
    }

    #[route(GET "/jwks")]
    async fn keys(cx: &Cx) -> TopcoatResult<Response> {
        let state = app_context::<Arc<ServerState>>(cx);
        state.requests.fetch_add(1, Ordering::SeqCst);
        state
            .saw_etag
            .store(headers(cx).get(IF_NONE_MATCH).is_some(), Ordering::SeqCst);
        state.saw_last_modified.store(
            headers(cx).get(IF_MODIFIED_SINCE).is_some(),
            Ordering::SeqCst,
        );

        let response = match state.mode.load(Ordering::SeqCst) {
            OK => Response::builder()
                .header(ETAG, "\"one\"")
                .header(LAST_MODIFIED, LAST_CHANGED)
                .body(Body::from(state.body.lock().unwrap().clone()))?,
            NOT_MODIFIED => Response::builder()
                .status(StatusCode::NOT_MODIFIED)
                .header(ETAG, "\"one\"")
                .header(LAST_MODIFIED, LAST_CHANGED)
                .body(Body::empty())?,
            UNAVAILABLE => Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .body(Body::empty())?,
            _ => unreachable!(),
        };
        Ok(response)
    }

    async fn server() -> Server {
        let state = Arc::new(ServerState {
            mode: AtomicU8::new(OK),
            requests: AtomicUsize::new(0),
            saw_etag: AtomicBool::new(false),
            saw_last_modified: AtomicBool::new(false),
            body: Mutex::new(JWKS.to_owned()),
        });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let router = Router::builder()
            .route(keys)
            .app_context(state.clone())
            .build();
        let task = tokio::spawn(async move {
            topcoat::serve_until(listener, router, std::future::pending())
                .await
                .unwrap();
        });

        Server {
            state,
            url: Url::parse(&format!("http://{address}/jwks")).unwrap(),
            task,
        }
    }

    async fn expire(jwks: &Jwks) {
        jwks.state.write().await.validated_at = Instant::now() - MAX_AGE;
    }

    #[tokio::test]
    async fn cached_and_unknown_keys_do_not_fetch() {
        let server = server().await;
        let jwks = Jwks::new(Client::new(), server.url.clone()).await.unwrap();

        assert!(jwks.key("one").await.unwrap().is_some());
        assert!(jwks.key("missing").await.unwrap().is_none());
        assert_eq!(server.state.requests.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn stale_keys_revalidate_with_http_validators() {
        let server = server().await;
        let jwks = Jwks::new(Client::new(), server.url.clone()).await.unwrap();
        server.state.mode.store(NOT_MODIFIED, Ordering::SeqCst);
        expire(&jwks).await;

        assert!(jwks.key("one").await.unwrap().is_some());
        assert_eq!(server.state.requests.load(Ordering::SeqCst), 2);
        assert!(server.state.saw_etag.load(Ordering::SeqCst));
        assert!(server.state.saw_last_modified.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn stale_keys_fail_closed_and_pause_before_retrying() {
        let server = server().await;
        let jwks = Jwks::new(Client::new(), server.url.clone()).await.unwrap();
        server.state.mode.store(UNAVAILABLE, Ordering::SeqCst);
        expire(&jwks).await;

        assert!(jwks.key("one").await.is_err());
        assert_eq!(server.state.requests.load(Ordering::SeqCst), 4);
        assert!(jwks.key("one").await.is_err());
        assert_eq!(server.state.requests.load(Ordering::SeqCst), 4);
    }

    #[tokio::test]
    async fn oversized_responses_are_rejected() {
        let server = server().await;
        *server.state.body.lock().unwrap() = "x".repeat(MAX_RESPONSE_BYTES + 1);

        assert!(Jwks::new(Client::new(), server.url.clone()).await.is_err());
        assert_eq!(server.state.requests.load(Ordering::SeqCst), 1);
    }
}
