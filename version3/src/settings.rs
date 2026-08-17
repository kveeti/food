use topcoat::{
    Result,
    context::Cx,
    router::{
        IntoResponse, Response, RouterBuilder,
        content::Form,
        error::{bad_request, see_other},
        page, route,
    },
    view::view,
};

use crate::{auth, db};

pub fn register(builder: RouterBuilder) -> RouterBuilder {
    builder.page(settings).route(save_timezone)
}

#[page("/settings")]
async fn settings(cx: &Cx) -> Result {
    let user = auth::require_user(cx).await?;
    let timezones: Vec<String> = sqlx::query_scalar(
        "SELECT name
         FROM pg_timezone_names
         WHERE name !~ '^(posix|right)/'
           AND name NOT IN ('Factory', 'localtime')
         ORDER BY name",
    )
    .fetch_all(db(cx))
    .await?;

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

                <form method="post" action="/settings/timezone" class="space-y-3">
                    <label for="timezone" class="sr-only">"Timezone"</label>
                    <select
                        id="timezone"
                        name="timezone"
                        required="true"
                        class="w-full rounded-lg border border-gray-300 bg-form px-3 py-2.5 text-base text-gray-1000 outline-none hover:border-gray-400 focus:border-gray-500 focus:ring-1 focus:ring-gray-400"
                    >
                        if user.timezone.is_none() {
                            <option value="" selected="true" disabled="true">"Choose a timezone"</option>
                        }
                        for timezone in &timezones {
                            if user.timezone.as_deref() == Some(timezone.as_str()) {
                                <option value=(timezone) selected="true">(timezone)</option>
                            } else {
                                <option value=(timezone)>(timezone)</option>
                            }
                        }
                    </select>
                    <button
                        type="submit"
                        class="rounded-lg bg-gray-900 px-4 py-2.5 text-base font-medium text-white hover:bg-gray-800 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-gray-700 active:bg-gray-1000"
                    >
                        "Save"
                    </button>
                </form>
            </section>
        </main>
    }
}

#[derive(Debug, serde::Deserialize)]
struct TimezoneForm {
    timezone: String,
}

#[route(POST "/settings/timezone")]
async fn save_timezone(cx: &Cx, Form(input): Form<TimezoneForm>) -> Result<Response> {
    let user = auth::require_user(cx).await?;
    let timezone: Option<String> = sqlx::query_scalar(
        "SELECT name
         FROM pg_timezone_names
         WHERE name = $1
           AND name !~ '^(posix|right)/'
           AND name NOT IN ('Factory', 'localtime')",
    )
    .bind(&input.timezone)
    .fetch_optional(db(cx))
    .await?;
    let timezone = timezone.ok_or_else(|| bad_request("unknown timezone"))?;

    sqlx::query("UPDATE users SET timezone = $1 WHERE id = $2")
        .bind(timezone)
        .bind(user.id)
        .execute(db(cx))
        .await?;

    Ok(see_other("/").into_response(cx)?)
}
