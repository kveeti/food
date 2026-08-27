mod admin;
mod app;
mod auth;
mod components;
mod day;
mod dev_oidc;
mod food;
mod goals;
mod pwa;
mod settings;
mod water;

use std::time::Duration;

use sqlx::PgPool;
use topcoat::{
    asset::{AssetBundle, RouterBuilderAssetExt},
    context::{Cx, app_context},
    cookie::RouterBuilderCookieExt,
    font::RouterBuilderFontExt,
    router::Router,
    session::{RouterBuilderSessionExt, SessionConfig},
};

pub(crate) fn db(cx: &Cx) -> &PgPool {
    app_context(cx)
}

#[tokio::main]
async fn main() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&database_url).await.unwrap();

    sqlx::raw_sql(include_str!("../migrations/001_initial.sql"))
        .execute(&pool)
        .await
        .unwrap();
    let timezone_catalog = settings::TimezoneCatalog::load(&pool).await.unwrap();

    let auth = auth::Auth::from_env();
    let import_config = admin::ImportConfig::from_env(auth.is_prod);
    let dev_provider = (!auth.is_prod).then(|| dev_oidc::DevOidc::new(&auth));
    let session_config = SessionConfig::builder().lifetime(Duration::from_secs(7 * 24 * 60 * 60));
    let session_config = if auth.is_prod {
        session_config
    } else {
        session_config.token_store(auth::DevCookieTokenStore)
    };
    let builder = Router::builder()
        .font(app::GEIST)
        .cookies()
        .sessions(session_config.build())
        .assets(AssetBundle::load().unwrap())
        .app_context(pool.clone())
        .app_context(auth)
        .app_context(import_config.clone())
        .app_context(timezone_catalog);
    let builder = admin::register(builder);
    let builder = app::register(builder);
    let builder = auth::register(builder);
    let builder = food::register(builder);
    let builder = goals::register(builder);
    let builder = pwa::register(builder);
    let builder = settings::register(builder);
    let builder = water::register(builder);
    let builder = if let Some(dev_provider) = dev_provider {
        dev_oidc::register(builder).app_context(dev_provider)
    } else {
        builder
    };

    tokio::select! {
        result = admin::run_worker(pool, import_config) => {
            if let Err(error) = result {
                panic!("import worker stopped: {error}");
            }
            std::future::pending::<()>().await;
        }
        result = topcoat::start(builder.build()) => {
            result.unwrap();
        }
    }
}
