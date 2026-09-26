use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{
        RouterBuilder,
        content::Json,
        error::{bad_request, not_found},
        path_param, query_params, route,
    },
};
use uuid::Uuid;

use crate::{
    auth,
    data::{
        Data,
        weight::{WeightEntry, WeightPoint},
    },
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

#[topcoat::router::query_params(error = bad_request)]
struct EntriesQuery {
    limit: Option<u32>,
}

#[topcoat::router::query_params(error = bad_request)]
struct ChartQuery {
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    max_points: u32,
    timezone: String,
}

#[derive(Serialize)]
struct ChartResponse {
    points: Vec<WeightPoint>,
    first: Option<WeightEntry>,
    last: Option<WeightEntry>,
    latest: Option<WeightEntry>,
}

path_param!(entry_id: Uuid, error = bad_request);

pub fn register(builder: RouterBuilder) -> RouterBuilder {
    builder
        .route(entries)
        .route(chart_entries)
        .route(add)
        .route(update)
        .route(delete)
}

#[route(GET "/api/weight-entries")]
#[tracing::instrument(name = "api::weight::entries", level = "debug", skip_all)]
async fn entries(cx: &Cx) -> Result<Json<Vec<WeightEntry>>> {
    let user = auth::require_user(cx).await?;
    let query = query_params::<EntriesQuery>(cx)?;
    let limit = query.limit.unwrap_or(10);
    if !(1..=100).contains(&limit) {
        return Err(bad_request("limit must be between 1 and 100").into());
    }
    Ok(Json(
        app_context::<Data>(cx)
            .recent_weight_entries(user.id, i64::from(limit))
            .await?,
    ))
}

#[route(GET "/api/weight-chart")]
#[tracing::instrument(name = "api::weight::chart", level = "debug", skip_all)]
async fn chart_entries(cx: &Cx) -> Result<Json<ChartResponse>> {
    let user = auth::require_user(cx).await?;
    let query = query_params::<ChartQuery>(cx)?;
    if !(2..=2_000).contains(&query.max_points) {
        return Err(bad_request("max_points must be between 2 and 2000").into());
    }
    if query.timezone.parse::<chrono_tz::Tz>().is_err() {
        return Err(bad_request("invalid timezone").into());
    }
    let data = app_context::<Data>(cx);
    Ok(Json(ChartResponse {
        points: data
            .weight_chart_points(
                user.id,
                query.from,
                query.to,
                i64::from(query.max_points),
                &query.timezone,
            )
            .await?,
        first: data
            .first_weight_entry_since(user.id, query.from, query.to)
            .await?,
        last: data
            .last_weight_entry_in_range(user.id, query.from, query.to)
            .await?,
        latest: data.latest_weight_entry(user.id).await?,
    }))
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
