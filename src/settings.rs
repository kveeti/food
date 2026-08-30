use chrono_tz::{TZ_VARIANTS, Tz};
use language_tags::LanguageTag;
use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{
        RouterBuilder, Uri,
        content::Form,
        error::{bad_request, see_other},
        page, query_params,
        response::{IntoResponse, Response},
        route,
    },
    view::view,
};

use crate::{auth, data::Data};

pub fn register(builder: RouterBuilder) -> RouterBuilder {
    builder.page(settings).route(save_settings)
}

pub fn setup_url(return_to: &str) -> String {
    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("return_to", return_to)
        .finish();
    format!("/settings?{query}")
}

#[topcoat::router::query_params(error = bad_request)]
struct SettingsQuery {
    return_to: Option<String>,
}

#[derive(Deserialize)]
struct SettingsForm {
    locale: String,
    timezone: String,
    return_to: String,
}

#[page("/settings")]
#[tracing::instrument(name = "settings::settings", level = "debug", skip_all)]
async fn settings(cx: &Cx) -> Result {
    let user = auth::require_user(cx).await?;
    let query = query_params::<SettingsQuery>(cx)?;
    let return_to = safe_return_to(query.return_to.as_deref());
    let locale = user.locale.as_deref().unwrap_or_default();
    let timezone = user.timezone.as_deref().unwrap_or_default();

    view! {
        <main>
            <h1>"Settings"</h1>
            if !user.has_settings() {
                <p>"Choose your locale and timezone to continue."</p>
            }
            <form
                method="post"
                action="/settings"
                hx-boost="true"
                hx-target="#app"
                hx-swap="outerHTML"
            >
                <input type="hidden" name="return_to" value=(return_to)>

                <p>
                    <label for="locale">"Locale"</label>
                    <input
                        id="locale"
                        name="locale"
                        value=(locale)
                        placeholder="en-US"
                        autocomplete="language"
                        required="true"
                    >
                </p>

                <p>
                    <label for="timezone">"Timezone"</label>
                    <input
                        id="timezone"
                        name="timezone"
                        value=(timezone)
                        placeholder="Europe/Helsinki"
                        autocomplete="off"
                        list="timezones"
                        required="true"
                    >
                    <datalist id="timezones">
                        for timezone in TZ_VARIANTS.iter() {
                            <option value=(timezone.name())></option>
                        }
                    </datalist>
                </p>

                <p id="device-settings-hint" hidden="true">
                    "Suggested from this device."
                </p>
                <button type="submit">"Save"</button>
            </form>
        </main>
    }
}

#[route(POST "/settings")]
#[tracing::instrument(name = "settings::save_settings", level = "debug", skip_all)]
async fn save_settings(cx: &Cx, Form(input): Form<SettingsForm>) -> Result<Response> {
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
    let return_to = if user.has_settings() {
        "/settings".to_owned()
    } else {
        safe_return_to(Some(&input.return_to))
    };

    data(cx)
        .save_user_settings(user.id, locale.as_str(), timezone.name())
        .await?;

    see_other(&return_to).into_response(cx)
}

fn data(cx: &Cx) -> &Data {
    app_context(cx)
}

fn safe_return_to(value: Option<&str>) -> String {
    let Some(value) = value else {
        return "/".to_owned();
    };
    let Ok(uri) = value.parse::<Uri>() else {
        return "/".to_owned();
    };
    if uri.scheme().is_some()
        || uri.authority().is_some()
        || !uri.path().starts_with('/')
        || uri.path().starts_with("//")
        || uri.path().contains('\\')
    {
        return "/".to_owned();
    }
    uri.path_and_query()
        .map_or_else(|| "/".to_owned(), |value| value.as_str().to_owned())
}
