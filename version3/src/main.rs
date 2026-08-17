use chrono::{DateTime, Utc};
use sqlx::{PgPool, types::Uuid};

use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt, asset},
    context::{Cx, app_context},
    font::{Font, RouterBuilderFontExt, font},
    htmx::hx_request,
    router::{
        IntoResponse, Response, Router,
        content::Form,
        error::{bad_request, see_other},
        layout, page, path_param, route,
    },
    tailwind,
    view::view,
};

const GEIST: Font = font! {
    "Geist",
    @font-face {
        src: url(asset!("public/fonts/geist.woff2")) format("woff2");
        font-style: normal;
        font-weight: 100 900;
        font-display: swap;
    }
};

fn db(cx: &Cx) -> &PgPool {
    app_context(cx)
}

#[tokio::main]
async fn main() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&database_url).await.unwrap();

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS water_log (
            id          UUID PRIMARY KEY DEFAULT uuidv7(),
            amount_ml   INTEGER NOT NULL CHECK (amount_ml > 0),
            consumed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
            created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
        )",
    )
    .execute(&pool)
    .await
    .unwrap();

    let router = Router::builder()
        .layout(root_layout)
        .page(home)
        .route(log_water)
        .route(delete_water)
        .font(GEIST)
        .assets(AssetBundle::load().unwrap())
        .app_context(pool)
        .build();

    topcoat::start(router).await.unwrap();
}

#[layout("/")]
async fn root_layout(slot: Result) -> Result {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <meta name="theme-color" content="oklch(98.8% 0.003 70deg)" media="(prefers-color-scheme: light)">
                <meta name="theme-color" content="oklch(16.74% 0 none)" media="(prefers-color-scheme: dark)">
                <meta name="htmx-config" content=(r#"{"extensions":"hx-optimistic"}"#)>
                <title>"Food"</title>

                topcoat::dev::script()
                topcoat::font::link(font: GEIST)
                <link rel="stylesheet" href=(tailwind::stylesheet!())>
                <script defer="true" src=(asset!("https://cdn.jsdelivr.net/npm/htmx.org@4.0.0-beta6/dist/htmx.min.js"))></script>
                <script defer="true" src=(asset!("public/hx-optimistic.js"))></script>
            </head>

            <body class="min-h-screen bg-canvas font-sans text-gray-950 antialiased">
                <nav class="fixed inset-x-0 bottom-0 z-10 h-(--nav-height) bg-nav/80 backdrop-blur-md sm:sticky sm:top-0">
                    <div class="mx-auto flex h-full w-full max-w-(--page-width) items-stretch px-3 sm:px-6">
                        <a
                            href="/"
                            aria-current="page"
                            class="inline-flex h-full items-center px-3 text-sm text-gray-950 underline decoration-1 underline-offset-3 hover:bg-gray-200/70 sm:text-base"
                        >
                            "today"
                        </a>
                    </div>
                </nav>

                (slot?)
            </body>
        </html>
    }
}

#[derive(Debug, serde::Deserialize)]
struct LogWater {
    amount_ml: i32,
}

#[derive(Debug)]
struct WaterLog {
    id: Uuid,
    amount_ml: i32,
    consumed_at: DateTime<Utc>,
}

#[page("/")]
async fn home(cx: &Cx) -> Result {
    let (entries, total_today) = load_water_log(db(cx)).await?;

    view! {
        <main class="mx-auto w-full max-w-(--page-width) px-3 pb-[calc(var(--nav-height)+2rem)] pt-6 sm:px-6 sm:pb-12 sm:pt-10">
            <header class="mb-8">
                <p class="mb-1 text-sm text-gray-600">"Today"</p>
                <h1 class="text-xl font-semibold tracking-tight text-gray-1000">"Water"</h1>
            </header>

            <section aria-labelledby="water-total" class="mb-8">
                <h2 id="water-total" class="sr-only">"Water total"</h2>
                <p class="tabular-nums text-[2.5rem] font-semibold leading-none tracking-tight text-gray-1000">
                    <span id="today-total">(total_today)</span>
                    <span class="ms-2 text-base font-normal text-gray-600">"ml today"</span>
                </p>
            </section>

            <section aria-labelledby="quick-add-heading" class="mb-10">
                <h2 id="quick-add-heading" class="mb-3 text-sm font-medium text-gray-700">"Quick add"</h2>

                <div class="grid grid-cols-2 gap-2">
                    water_button(amount_ml: 250)
                    water_button(amount_ml: 750)
                </div>

                <form
                    method="post"
                    action="/water"
                    class="mt-2 flex gap-2"
                    hx-post="/water"
                    hx-target="#entries"
                    hx-swap="afterbegin"
                    hx-optimistic="#water-optimistic"
                >
                    <label for="custom-water" class="sr-only">"Water in millilitres"</label>
                    <input
                        id="custom-water"
                        name="amount_ml"
                        type="number"
                        inputmode="numeric"
                        min="1"
                        max="10000"
                        required="true"
                        placeholder="Other amount"
                        class="min-w-0 flex-1 rounded-lg border border-gray-300 bg-form px-3 py-2 text-base text-gray-1000 outline-none placeholder:text-gray-500 hover:border-gray-400 focus:border-gray-500 focus:ring-1 focus:ring-gray-400"
                    >
                    <button
                        type="submit"
                        class="rounded-lg bg-gray-900 px-4 py-2 text-base font-medium text-white hover:bg-gray-800 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-gray-700 active:bg-gray-1000"
                    >
                        "Add"
                    </button>
                </form>
            </section>

            <template id="water-optimistic">
                <li class="flex min-h-11 items-center justify-between border-t border-gray-200 px-1 py-2 text-base opacity-50 first:border-t-0">
                    <span class="optimistic-amount font-medium text-gray-900"></span>
                    <span class="text-sm text-gray-500">"adding..."</span>
                </li>
            </template>

            log_entries_component(entries: &entries)
        </main>
    }
}

#[topcoat::view::component]
async fn water_button(amount_ml: i32) -> Result {
    view! {
        <form
            method="post"
            action="/water"
            hx-post="/water"
            hx-target="#entries"
            hx-swap="afterbegin"
            hx-optimistic="#water-optimistic"
        >
            <input type="hidden" name="amount_ml" value=(amount_ml)>
            <button
                type="submit"
                class="w-full rounded-lg bg-gray-900 px-4 py-3 text-base font-medium text-white hover:bg-gray-800 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-gray-700 active:bg-gray-1000"
            >
                (amount_ml) " ml"
            </button>
        </form>
    }
}

#[route(POST "/water")]
async fn log_water(cx: &Cx, Form(input): Form<LogWater>) -> Result<Response> {
    if !(1..=10_000).contains(&input.amount_ml) {
        return Err(bad_request("water must be between 1 and 10000 ml").into());
    }

    let (id, amount_ml, consumed_at) = sqlx::query_as(
        "INSERT INTO water_log (amount_ml)
         VALUES ($1)
         RETURNING id, amount_ml, consumed_at",
    )
    .bind(input.amount_ml)
    .fetch_one(db(cx))
    .await?;
    let entry = WaterLog {
        id,
        amount_ml,
        consumed_at,
    };

    if !hx_request(cx) {
        return see_other("/").into_response(cx);
    }

    let total_today = load_today_total(db(cx)).await?;
    let fragment = view! {
        water_entry(entry: &entry)
        <span id="today-total" hx-swap-oob="true">(total_today)</span>
        <li id="empty-state" hx-swap-oob="delete"></li>
    }?;

    fragment.into_response(cx)
}

#[path_param(error = bad_request)]
struct Id(Uuid);

#[route(POST "/water/{id}/delete")]
async fn delete_water(cx: &Cx) -> Result<Response> {
    let id = path_param::<Id>(cx)?;

    sqlx::query("DELETE FROM water_log WHERE id = $1")
        .bind(id)
        .execute(db(cx))
        .await?;

    if !hx_request(cx) {
        return see_other("/").into_response(cx);
    }

    let (entries, total_today) = load_water_log(db(cx)).await?;
    let fragment = view! {
        log_entries_component(entries: &entries)
        <span id="today-total" hx-swap-oob="true">(total_today)</span>
    }?;

    fragment.into_response(cx)
}

async fn load_water_log(pool: &PgPool) -> Result<(Vec<WaterLog>, i64)> {
    let (rows, total_today) = tokio::try_join!(
        sqlx::query_as(
            "SELECT id, amount_ml, consumed_at
             FROM water_log
             ORDER BY consumed_at DESC
             LIMIT 20",
        )
        .fetch_all(pool),
        load_today_total(pool),
    )?;
    let entries = rows
        .into_iter()
        .map(|(id, amount_ml, consumed_at)| WaterLog {
            id,
            amount_ml,
            consumed_at,
        })
        .collect();

    Ok((entries, total_today))
}

async fn load_today_total(pool: &PgPool) -> sqlx::Result<i64> {
    Ok(sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount_ml), 0)
         FROM water_log
         WHERE consumed_at >= (now() AT TIME ZONE 'UTC')::date",
    )
    .fetch_one(pool)
    .await?)
}

#[topcoat::view::component]
async fn log_entries_component(entries: &[WaterLog]) -> Result {
    view! {
        <section id="log-list" aria-labelledby="history-heading">
            <div class="mb-2 flex items-baseline justify-between">
                <h2 id="history-heading" class="text-sm font-medium text-gray-700">"Recent"</h2>
            </div>
            <ul id="entries" class="border-y border-gray-200">
                if entries.is_empty() {
                    <li id="empty-state" class="py-8 text-center text-sm text-gray-500">"No water logged yet"</li>
                } else {
                    for entry in entries {
                        water_entry(entry: entry)
                    }
                }
            </ul>
        </section>
    }
}

#[topcoat::view::component]
async fn water_entry(entry: &WaterLog) -> Result {
    view! {
        <li class="border-t border-gray-200 first:border-t-0">
            <form
                method="post"
                action=(format!("/water/{}/delete", entry.id))
                class="flex min-h-11 items-center justify-between px-1 py-2"
                hx-post=(format!("/water/{}/delete", entry.id))
                hx-target="#log-list"
                hx-swap="outerHTML"
            >
                <span class="font-medium tabular-nums text-gray-900">(entry.amount_ml) " ml"</span>
                <span class="flex items-center gap-3">
                    <time class="text-sm tabular-nums text-gray-500" datetime=(entry.consumed_at.to_rfc3339())>
                        (entry.consumed_at.format("%H:%M").to_string())
                    </time>
                    <button
                        type="submit"
                        aria-label=(format!("Delete {} ml entry", entry.amount_ml))
                        class="grid size-8 place-items-center rounded-lg text-gray-500 hover:bg-red-100 hover:text-red-700 focus-visible:outline-2 focus-visible:outline-red-500"
                    >
                        <svg aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" class="size-4">
                            <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12"></path>
                        </svg>
                    </button>
                </span>
            </form>
        </li>
    }
}
