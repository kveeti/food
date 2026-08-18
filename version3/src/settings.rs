use sqlx::PgPool;
use topcoat::{
    Result,
    asset::{Asset, asset},
    context::{Cx, app_context},
    router::{
        IntoResponse, Response, RouterBuilder,
        content::Form,
        error::{bad_request, see_other},
        page, query_params, route,
    },
    view::view,
};

use crate::{auth, db, ui::search_spinner};

pub const SETTINGS_JS: Asset = asset!("public/settings.js");

pub struct TimezoneCatalog {
    names: Vec<String>,
}

impl TimezoneCatalog {
    pub async fn load(pool: &PgPool) -> sqlx::Result<Self> {
        let names = sqlx::query_scalar(
            "SELECT name
             FROM pg_timezone_names
             WHERE name !~ '^(posix|right)/'
               AND name NOT IN ('Factory', 'localtime')
             ORDER BY name",
        )
        .fetch_all(pool)
        .await?;

        Ok(Self { names })
    }

    fn contains(&self, timezone: &str) -> bool {
        self.names.iter().any(|name| name == timezone)
    }

    fn search(&self, query: &str) -> Vec<&str> {
        let query = query.to_lowercase();
        let starts_with = self
            .names
            .iter()
            .filter(|name| name.to_lowercase().starts_with(&query));
        let contains = self.names.iter().filter(|name| {
            let name = name.to_lowercase();
            !name.starts_with(&query) && name.contains(&query)
        });

        starts_with
            .chain(contains)
            .take(8)
            .map(String::as_str)
            .collect()
    }
}

fn catalog(cx: &Cx) -> &TimezoneCatalog {
    app_context(cx)
}

pub fn register(builder: RouterBuilder) -> RouterBuilder {
    builder
        .page(settings)
        .route(search_timezones)
        .route(save_timezone)
}

#[topcoat::router::query_params(error = bad_request)]
struct SettingsQuery {
    required: Option<String>,
}

#[page("/settings")]
async fn settings(cx: &Cx) -> Result {
    let user = auth::require_user(cx).await?;
    let query = query_params::<SettingsQuery>(cx)?;
    let required = query.required.as_deref() == Some("1") && user.timezone.is_none();
    let timezone = user.timezone.unwrap_or_default();

    view! {
        <main class="mx-auto w-full max-w-(--page-width) px-3 pb-[calc(var(--nav-height)+2rem)] pt-8 sm:px-6 sm:pb-12 sm:pt-12">
            <header class="mb-8">
                <p class="mb-1 text-sm text-gray-600">"Account"</p>
                <h1 class="text-xl font-semibold tracking-tight text-gray-1000">"Settings"</h1>
            </header>

            <section aria-labelledby="timezone-heading">
                <h2 id="timezone-heading" class="mb-1 text-base font-medium text-gray-1000">"Timezone"</h2>
                <p class="mb-4 text-sm leading-6 text-gray-600">
                    "Days and entry times use this timezone."
                </p>

                <form id="timezone-form" method="post" action="/settings/timezone" class="space-y-3">
                    if required {
                        <input type="hidden" name="required" value="1">
                    }
                    <label for="timezone-query" class="sr-only">"Timezone"</label>
                    <div class="flex gap-2">
                        <input
                            id="timezone-query"
                            name="query"
                            type="search"
                            value=(timezone)
                            required="true"
                            autocomplete="off"
                            spellcheck="false"
                            placeholder="Europe/Helsinki"
                            aria-controls="timezone-results"
                            aria-describedby="device-timezone-hint"
                            hx-get="/settings/timezones"
                            hx-trigger="input changed delay:100ms, search"
                            hx-target="#timezone-results"
                            hx-swap="innerHTML"
                            hx-sync="closest form:replace"
                            hx-indicator="#timezone-search"
                            class="min-w-0 flex-1 rounded-lg border border-gray-300 bg-form px-3 py-2.5 text-base text-gray-1000 outline-none placeholder:text-gray-500 hover:border-gray-400 focus:border-gray-500 focus:ring-1 focus:ring-gray-400"
                        >
                        <button
                            id="timezone-search"
                            type="button"
                            hx-get="/settings/timezones"
                            hx-include="#timezone-query"
                            hx-target="#timezone-results"
                            hx-swap="innerHTML"
                            hx-sync="closest form:replace"
                            hx-indicator="#timezone-search"
                            class="relative min-w-23 rounded-lg border border-gray-200 px-3 py-2 text-sm font-medium text-gray-800 hover:bg-gray-150 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-gray-700"
                        >
                            <span class="search-label">"Search"</span>
                            search_spinner()
                        </button>
                    </div>
                    <p id="device-timezone-hint" hidden="true" class="text-sm text-gray-500">
                        "Suggested from this device."
                    </p>

                    <div id="timezone-results" aria-live="polite"></div>

                    <button
                        type="submit"
                        class="rounded-lg border border-gray-200 px-3 py-2 text-sm font-medium text-gray-800 hover:bg-gray-150 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-gray-700"
                    >
                        "Save"
                    </button>
                </form>
            </section>
        </main>
    }
}

#[topcoat::router::query_params(error = bad_request)]
struct TimezoneSearch {
    query: Option<String>,
}

#[route(GET "/settings/timezones")]
async fn search_timezones(cx: &Cx) -> Result {
    auth::require_user(cx).await?;
    let query = query_params::<TimezoneSearch>(cx)?;
    let query = query.query.as_deref().unwrap_or_default().trim();
    let matches = if query.is_empty() {
        Vec::new()
    } else {
        catalog(cx).search(query)
    };

    view! {
        if !query.is_empty() {
            if matches.is_empty() {
                <p class="py-2 text-sm text-gray-500">"No matching timezones"</p>
            } else {
                <ul class="space-y-2">
                    for timezone in matches {
                        <li>
                            <button
                                type="submit"
                                name="timezone"
                                value=(timezone)
                                class="w-full rounded-lg border border-gray-200 px-3 py-2 text-left text-sm font-medium text-gray-800 hover:bg-gray-150 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-gray-700"
                            >
                                (timezone)
                            </button>
                        </li>
                    }
                </ul>
            }
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct TimezoneForm {
    query: String,
    timezone: Option<String>,
    required: Option<String>,
}

#[route(POST "/settings/timezone")]
async fn save_timezone(cx: &Cx, Form(input): Form<TimezoneForm>) -> Result<Response> {
    let user = auth::require_user(cx).await?;
    let timezone = input.timezone.as_deref().unwrap_or(&input.query).trim();
    if !catalog(cx).contains(timezone) {
        return Err(bad_request("unknown timezone").into());
    }

    sqlx::query("UPDATE users SET timezone = $1 WHERE id = $2")
        .bind(timezone)
        .bind(user.id)
        .execute(db(cx))
        .await?;

    let location = if input.required.as_deref() == Some("1") {
        "/"
    } else {
        "/settings"
    };
    Ok(see_other(location).into_response(cx)?)
}
