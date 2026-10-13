use std::{env, time::Duration};

use anyhow::{Result, anyhow};
use opentelemetry::{KeyValue, global, trace::TracerProvider as _};
use opentelemetry_otlp::{
    WithExportConfig as _, WithTonicConfig as _,
    tonic_types::{metadata::MetadataMap, transport::ClientTlsConfig},
};
use opentelemetry_sdk::{
    Resource,
    propagation::TraceContextPropagator,
    trace::{Sampler, SdkTracerProvider},
};
use opentelemetry_semantic_conventions::{SCHEMA_URL, attribute::SERVICE_VERSION};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{
    EnvFilter, filter::LevelFilter, layer::SubscriberExt as _, util::SubscriberInitExt as _,
};

const MAX_EVENTS_PER_SPAN: u32 = 64 * 1024;
const MAX_ATTRIBUTES_PER_SPAN: u32 = 128;

pub struct Telemetry(Option<SdkTracerProvider>);

impl Telemetry {
    pub fn start() -> Result<Self> {
        global::set_text_map_propagator(TraceContextPropagator::new());
        let filter = EnvFilter::builder()
            .with_default_directive(LevelFilter::INFO.into())
            .from_env_lossy();
        let Some(endpoint) = env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
            .ok()
            .filter(|endpoint| !endpoint.is_empty())
        else {
            tracing_subscriber::registry()
                .with(filter)
                .with(tracing_forest::ForestLayer::default())
                .init();
            return Ok(Self(None));
        };

        eprintln!("Starting OTLP tracing endpoint={endpoint}");
        let mut metadata = MetadataMap::new();
        if let Some(headers) = env::var_os("OTEL_EXPORTER_OTLP_HEADERS") {
            for header in headers.to_string_lossy().split(',') {
                if let Some((name, value)) = header.split_once('=')
                    && name.eq_ignore_ascii_case("authorization")
                {
                    let value = value
                        .parse()
                        .map_err(|_| anyhow!("OTEL authorization header is invalid"))?;
                    metadata.insert("authorization", value);
                }
            }
        }

        let is_https = endpoint.starts_with("https://");
        let mut exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .with_endpoint(endpoint)
            .with_metadata(metadata)
            .with_timeout(Duration::from_secs(5));
        if is_https {
            exporter = exporter.with_tls_config(ClientTlsConfig::new().with_webpki_roots());
        }
        let exporter = exporter.build()?;
        let service_name = env::var("OTEL_SERVICE_NAME")
            .ok()
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| "food".to_owned());
        let resource = Resource::builder()
            .with_schema_url(
                [KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION"))],
                SCHEMA_URL,
            )
            .with_service_name(service_name)
            .build();
        let provider = SdkTracerProvider::builder()
            .with_batch_exporter(exporter)
            .with_sampler(Sampler::ParentBased(Box::new(Sampler::AlwaysOn)))
            .with_max_events_per_span(MAX_EVENTS_PER_SPAN)
            .with_max_attributes_per_span(MAX_ATTRIBUTES_PER_SPAN)
            .with_resource(resource)
            .build();
        global::set_tracer_provider(provider.clone());
        let tracer = provider.tracer("food");

        tracing_subscriber::registry()
            .with(filter)
            .with(OpenTelemetryLayer::new(tracer))
            .init();

        Ok(Self(Some(provider)))
    }

    pub fn exports_otlp(&self) -> bool {
        self.0.is_some()
    }
}

impl Drop for Telemetry {
    fn drop(&mut self) {
        if let Some(provider) = self.0.take()
            && let Err(error) = provider.shutdown()
        {
            eprintln!("OTLP tracing shutdown failed: {error}");
        }
    }
}
