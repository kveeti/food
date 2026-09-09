mod food;
mod goals;
mod settings;
mod water;

use serde::Serialize;
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{RouterBuilder, content::Json, error::service_unavailable, route},
};

use crate::{auth, data::Data};

#[derive(Serialize)]
struct Health {
    status: &'static str,
}

#[derive(Serialize)]
struct Me {
    id: String,
    email: Option<String>,
    locale: Option<String>,
    timezone: Option<String>,
}

pub fn register(builder: RouterBuilder) -> RouterBuilder {
    let builder = builder.route(health).route(me);
    let builder = settings::register(builder);
    let builder = goals::register(builder);
    food::register(water::register(builder))
}

#[route(GET "/health")]
#[tracing::instrument(name = "api::health", level = "debug", skip_all)]
async fn health(cx: &Cx) -> Result<Json<Health>> {
    if let Err(error) = app_context::<Data>(cx).ping().await {
        tracing::error!(?error, "database health check failed");
        return Err(service_unavailable(1).into());
    }
    Ok(Json(Health { status: "ok" }))
}

#[route(GET "/api/me")]
#[tracing::instrument(name = "api::me", level = "debug", skip_all)]
async fn me(cx: &Cx) -> Result<Json<Me>> {
    let user = auth::require_user(cx).await?;
    Ok(Json(Me {
        id: user.id.to_string(),
        email: user.email,
        locale: user.locale,
        timezone: user.timezone,
    }))
}
