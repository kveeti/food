use chrono::NaiveDate;
use chrono_tz::Tz;
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
    data::{Data, WaterEntry as DataWaterEntry},
};

#[topcoat::router::query_params(error = bad_request)]
struct WaterQuery {
    date: String,
}

#[derive(Deserialize)]
struct AddWaterInput {
    id: String,
    amount_ml: i32,
    date: String,
}

#[derive(Serialize)]
struct WaterEntry {
    id: String,
    amount_ml: i32,
    consumed_at: chrono::DateTime<chrono::Utc>,
}

path_param!(entry_id: Uuid, error = bad_request);

pub fn register(builder: RouterBuilder) -> RouterBuilder {
    builder.route(entries).route(add).route(delete)
}

#[route(GET "/api/water-entries")]
#[tracing::instrument(name = "api::water::entries", level = "debug", skip_all)]
async fn entries(cx: &Cx) -> Result<Json<Vec<WaterEntry>>> {
    let user = auth::require_user(cx).await?;
    let query = query_params::<WaterQuery>(cx)?;
    let date = date(&query.date)?;
    let timezone = timezone(&user)?;
    let entries = data(cx)
        .water_entries(user.id, date, timezone.name())
        .await?
        .into_iter()
        .map(WaterEntry::from)
        .collect();

    Ok(Json(entries))
}

#[route(POST "/api/water-entries")]
#[tracing::instrument(name = "api::water::add", level = "debug", skip_all)]
async fn add(cx: &Cx, Json(input): Json<AddWaterInput>) -> Result<Json<WaterEntry>> {
    let user = auth::require_user(cx).await?;
    if !(10..=1_500).contains(&input.amount_ml) {
        return Err(bad_request("water must be between 10 and 1500 ml").into());
    }
    let id = input
        .id
        .parse()
        .map_err(|_| bad_request("water entry ID must be a UUID"))?;
    let date = date(&input.date)?;
    let timezone = timezone(&user)?;
    let entry = data(cx)
        .add_water(id, user.id, input.amount_ml, date, timezone.name())
        .await?;

    Ok(Json(entry.into()))
}

#[route(DELETE "/api/water-entries/{entry_id}")]
#[tracing::instrument(name = "api::water::delete", level = "debug", skip_all)]
async fn delete(cx: &Cx) -> Result<()> {
    let user = auth::require_user(cx).await?;
    let entry_id = path_param::<EntryId>(cx)?;
    if !data(cx).delete_water(user.id, *entry_id).await? {
        return Err(not_found().into());
    }
    Ok(())
}

impl From<DataWaterEntry> for WaterEntry {
    fn from(entry: DataWaterEntry) -> Self {
        Self {
            id: entry.id.to_string(),
            amount_ml: entry.amount_ml,
            consumed_at: entry.consumed_at,
        }
    }
}

fn date(value: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| bad_request("date must use YYYY-MM-DD").into())
}

fn timezone(user: &auth::User) -> Result<Tz> {
    let Some(timezone) = user.timezone.as_deref() else {
        return Err(bad_request("choose a locale and timezone first").into());
    };
    timezone
        .parse()
        .map_err(|_| bad_request("invalid user timezone").into())
}

fn data(cx: &Cx) -> &Data {
    app_context(cx)
}
