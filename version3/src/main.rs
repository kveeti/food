mod app;
mod auth;
mod day;
mod dev_oidc;
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

    let auth = auth::Auth::from_env();
    let dev_provider = (!auth.is_prod).then(|| dev_oidc::DevOidc::new(&auth));
    let builder = Router::builder()
        .font(app::GEIST)
        .cookies()
        .sessions(
            SessionConfig::builder()
                .lifetime(Duration::from_secs(7 * 24 * 60 * 60))
                .build(),
        )
        .assets(AssetBundle::load().unwrap())
        .app_context(pool)
        .app_context(auth);
    let builder = app::register(builder);
    let builder = auth::register(builder);
    let builder = pwa::register(builder);
    let builder = settings::register(builder);
    let builder = water::register(builder);
    let builder = if let Some(dev_provider) = dev_provider {
        dev_oidc::register(builder).app_context(dev_provider)
    } else {
        builder
    };

    topcoat::start(builder.build()).await.unwrap();
}
