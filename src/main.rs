mod app;
mod assets;
mod auth;
mod components;
mod config;
mod data;
mod http;
mod settings;
mod telemetry;

use std::sync::Arc;

use anyhow::Result;
use topcoat::{
    asset::{AssetBundle, RouterBuilderAssetExt},
    router::Router,
};

use crate::{config::Config, data::Data, telemetry::Telemetry};

#[tokio::main]
async fn main() -> Result<()> {
    let telemetry = Telemetry::start()?;
    let result = run(telemetry.exports_otlp()).await;
    if let Err(error) = &result {
        tracing::error!(?error, "Food stopped");
    }
    result
}

async fn run(exports_otlp: bool) -> Result<()> {
    let config = Config::from_env().map_err(anyhow::Error::msg)?;
    let data = Data::new(&config.database_url).await?;
    data.start_session_cleanup().await?;
    let auth = Arc::new(auth::Auth::new(&config).await?);

    let builder = Router::builder()
        .app_context(data)
        .app_context(auth)
        .assets(AssetBundle::load()?);
    let builder = http::register(builder, exports_otlp);
    let builder = assets::register(builder);
    let builder = app::register(builder);
    let builder = settings::register(builder);
    let builder = auth::routes::register(builder);

    tracing::info!(app_url = %config.app_url, "starting Food");
    topcoat::start(builder.build()).await?;
    Ok(())
}
