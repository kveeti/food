use chrono_tz::Tz;
use language_tags::LanguageTag;
use serde::{Deserialize, Serialize};
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{RouterBuilder, content::Json, error::bad_request, route},
};

use crate::{auth, data::Data};

#[derive(Deserialize)]
struct SettingsInput {
    locale: String,
    timezone: String,
}

#[derive(Serialize)]
struct Settings {
    locale: String,
    timezone: String,
}

pub fn register(builder: RouterBuilder) -> RouterBuilder {
    builder.route(save)
}

#[route(PUT "/api/settings")]
#[tracing::instrument(name = "api::settings::save", level = "debug", skip_all)]
async fn save(cx: &Cx, Json(input): Json<SettingsInput>) -> Result<Json<Settings>> {
    let user = auth::require_user(cx).await?;
    let locale =
        LanguageTag::parse(input.locale.trim()).map_err(|_| bad_request("invalid locale"))?;
    locale
        .validate()
        .map_err(|_| bad_request("invalid locale"))?;
    let timezone = input
        .timezone
        .trim()
        .parse::<Tz>()
        .map_err(|_| bad_request("unknown timezone"))?;

    data(cx)
        .save_user_settings(user.id, locale.as_str(), timezone.name())
        .await?;

    Ok(Json(Settings {
        locale: locale.as_str().to_owned(),
        timezone: timezone.name().to_owned(),
    }))
}

fn data(cx: &Cx) -> &Data {
    app_context(cx)
}
