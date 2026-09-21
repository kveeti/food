use chrono::{DateTime, Datelike, Utc};
use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{
        RouterBuilder,
        content::Json,
        error::{bad_request, not_found},
        path_param, route,
    },
};
use uuid::Uuid;

use crate::{
    auth,
    data::{Data, weight::WeightEntry},
};

#[derive(Deserialize)]
struct WeightInput {
    weight_kg: f64,
    measured_at: DateTime<Utc>,
}

impl WeightInput {
    fn validate(&self) -> Result<()> {
        if !self.weight_kg.is_finite() || self.weight_kg <= 0.0 {
            return Err(bad_request("weight must be a finite number greater than 0").into());
        }
        if !(1..=9999).contains(&self.measured_at.year()) {
            return Err(bad_request("measurement year must be between 1 and 9999").into());
        }
        Ok(())
    }
}

path_param!(entry_id: Uuid, error = bad_request);

pub fn register(builder: RouterBuilder) -> RouterBuilder {
    builder
        .route(entries)
        .route(add)
        .route(update)
        .route(delete)
}

#[route(GET "/api/weight-entries")]
#[tracing::instrument(name = "api::weight::entries", level = "debug", skip_all)]
async fn entries(cx: &Cx) -> Result<Json<Vec<WeightEntry>>> {
    let user = auth::require_user(cx).await?;
    Ok(Json(app_context::<Data>(cx).weight_entries(user.id).await?))
}

#[route(POST "/api/weight-entries")]
#[tracing::instrument(name = "api::weight::add", level = "debug", skip_all)]
async fn add(cx: &Cx, Json(input): Json<WeightInput>) -> Result<Json<WeightEntry>> {
    let user = auth::require_user(cx).await?;
    input.validate()?;
    Ok(Json(
        app_context::<Data>(cx)
            .add_weight(user.id, input.weight_kg, input.measured_at)
            .await?,
    ))
}

#[route(PATCH "/api/weight-entries/{entry_id}")]
#[tracing::instrument(name = "api::weight::update", level = "debug", skip_all)]
async fn update(cx: &Cx, Json(input): Json<WeightInput>) -> Result<Json<WeightEntry>> {
    let user = auth::require_user(cx).await?;
    input.validate()?;
    let id = path_param::<EntryId>(cx)?;
    Ok(Json(
        app_context::<Data>(cx)
            .update_weight(user.id, *id, input.weight_kg, input.measured_at)
            .await?
            .ok_or_else(not_found)?,
    ))
}

#[route(DELETE "/api/weight-entries/{entry_id}")]
#[tracing::instrument(name = "api::weight::delete", level = "debug", skip_all)]
async fn delete(cx: &Cx) -> Result<()> {
    let user = auth::require_user(cx).await?;
    let id = path_param::<EntryId>(cx)?;
    if !app_context::<Data>(cx).delete_weight(user.id, *id).await? {
        return Err(not_found().into());
    }
    Ok(())
}
