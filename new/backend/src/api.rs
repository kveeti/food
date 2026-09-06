use serde::Serialize;
use topcoat::{
    Result,
    context::Cx,
    router::{RouterBuilder, content::Json, route},
};

use crate::auth;

#[derive(Serialize)]
struct Me {
    id: String,
    email: Option<String>,
}

pub fn register(builder: RouterBuilder) -> RouterBuilder {
    builder.route(me)
}

#[route(GET "/api/me")]
#[tracing::instrument(name = "api::me", level = "debug", skip_all)]
async fn me(cx: &Cx) -> Result<Json<Me>> {
    let user = auth::require_user(cx).await?;
    Ok(Json(Me {
        id: user.id.to_string(),
        email: user.email,
    }))
}
