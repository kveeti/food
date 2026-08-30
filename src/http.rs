use opentelemetry::{
    propagation::{Extractor, TextMapPropagator as _},
    trace::Status,
};
use opentelemetry_sdk::propagation::TraceContextPropagator;
use topcoat::{
    context::{Cx, try_request_context},
    router::{
        Body, HeaderMap, Layer, LayerFuture, Next, Path, RouterBuilder,
        request::{headers, method, uri},
        response::IntoResponse as _,
        try_endpoint,
    },
};
use tracing::Instrument as _;
use tracing_opentelemetry::OpenTelemetrySpanExt as _;
use uuid::Uuid;

pub fn register(builder: RouterBuilder, exports_otlp: bool) -> RouterBuilder {
    builder.layer(RequestTracing { exports_otlp })
}

struct RequestTracing {
    exports_otlp: bool,
}

struct RequestSpan(tracing::Span);

struct HeaderExtractor<'a>(&'a HeaderMap);

impl Extractor for HeaderExtractor<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|value| value.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|name| name.as_str()).collect()
    }
}

fn external_request_id(cx: &Cx) -> Option<&str> {
    let value = headers(cx).get("x-request-id")?;
    let bytes = value.as_bytes();
    if bytes.is_empty()
        || bytes.len() > 128
        || !bytes.iter().all(|byte| matches!(byte, 0x21..=0x7e))
    {
        return None;
    }
    value.to_str().ok()
}

pub fn record_user(cx: &Cx, user_id: Uuid) {
    if let Some(span) = try_request_context::<RequestSpan>(cx) {
        span.0.record("enduser.id", user_id.to_string());
    }
}

impl Layer for RequestTracing {
    fn path(&self) -> Option<&Path> {
        None
    }

    fn handle<'a>(&'a self, cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        let request_id = match external_request_id(cx) {
            Some(request_id) => request_id.to_owned(),
            None => Uuid::now_v7().to_string(),
        };
        let method = method(cx).clone();
        let path = uri(cx).path().to_owned();
        let route = try_endpoint(cx).map(|endpoint| {
            let path = endpoint.path();
            if path.is_empty() {
                "/".to_owned()
            } else {
                path.to_string()
            }
        });

        let exports_otlp = self.exports_otlp;
        let otel_name = if exports_otlp {
            route.as_ref().map(|route| format!("{method} {route}"))
        } else {
            None
        };
        let span = tracing::info_span!(
            "http_request",
            request_id = %request_id,
            http.request.method = %method,
            url.path = %path,
            http.route = route,
            enduser.id = tracing::field::Empty,
            otel.name = otel_name,
            otel.kind = exports_otlp.then_some("server"),
            http.response.status_code = tracing::field::Empty,
        );
        if exports_otlp {
            let parent = TraceContextPropagator::new().extract(&HeaderExtractor(headers(cx)));
            if let Err(error) = span.set_parent(parent) {
                tracing::warn!(?error, "failed to set HTTP trace parent");
            }
        }

        let cx = cx.with(RequestSpan(span.clone()));
        Box::pin(
            async move {
                let result = next.run(&cx, body).await;

                let error = result.as_ref().err().map(|error| format!("{error:#}"));
                let mut response = result.into_response(&cx)?;
                let status = response.status();
                let span = tracing::Span::current();
                span.record("http.response.status_code", i64::from(status.as_u16()));
                if exports_otlp && status.is_server_error() {
                    span.set_status(Status::error(format!("HTTP {status}")));
                }

                if status.is_server_error() {
                    tracing::error!(
                        status = status.as_u16(),
                        error = error
                            .as_deref()
                            .map_or("response returned no error", |error| error),
                        "request failed"
                    );
                } else if status.is_client_error() {
                    tracing::warn!(status = status.as_u16(), "request rejected");
                }

                if let Ok(value) = request_id.parse() {
                    response.headers_mut().insert("x-request-id", value);
                }
                Ok(response)
            }
            .instrument(span),
        )
    }
}
