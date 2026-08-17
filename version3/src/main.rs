use chrono::{DateTime, Utc};
use sqlx::{PgPool, types::Uuid};

use topcoat::{
    Result,
    asset::{Asset, AssetBundle, RouterBuilderAssetExt, asset, asset_config},
    context::{Cx, app_context},
    font::{Font, RouterBuilderFontExt, font},
    htmx::hx_request,
    router::{
        HeaderValue, IntoResponse, Response, Router,
        content::Form,
        error::{bad_request, see_other},
        header, layout, page, path_param, route,
    },
    tailwind,
    view::view,
};

const APP_ICON: Asset = asset!("public/app-icon.svg");
const APP_ICON_192: Asset = asset!("public/app-icon-192.png");
const APP_ICON_512: Asset = asset!("public/app-icon-512.png");
const WORKBOX_CORE: Asset = asset!(
    "https://cdn.jsdelivr.net/npm/workbox-core@7.4.1/build/workbox-core.prod.js",
    checksum: "sha256:e5fe645c1c2019c0e32b5a863a03b484db2311d17df443704c025babc0bb1f4f"
);
const WORKBOX_ROUTING: Asset = asset!(
    "https://cdn.jsdelivr.net/npm/workbox-routing@7.4.1/build/workbox-routing.prod.js",
    checksum: "sha256:844e227e94979b17caa529a90bead72541c32985d91f1e17ed2589a741cf7647"
);
const WORKBOX_STRATEGIES: Asset = asset!(
    "https://cdn.jsdelivr.net/npm/workbox-strategies@7.4.1/build/workbox-strategies.prod.js",
    checksum: "sha256:c4139ddc2510aea347456a8ed167f820683492561394c64fd1eca5827cd219d3"
);

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
        .route(manifest)
        .route(service_worker)
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
                <link rel="manifest" href="/manifest.webmanifest">
                <link rel="icon" href=(APP_ICON) type="image/svg+xml">
                <link rel="apple-touch-icon" href=(APP_ICON_192)>
                <title>"Food"</title>

                topcoat::dev::script()
                topcoat::font::link(font: GEIST)
                <link rel="stylesheet" href=(tailwind::stylesheet!())>
                <script defer="true" src=(asset!("https://cdn.jsdelivr.net/npm/htmx.org@4.0.0-beta6/dist/htmx.min.js"))></script>
                <script defer="true" src=(asset!("public/hx-optimistic.js"))></script>
                <script defer="true" src=(asset!("public/register-service-worker.js"))></script>
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

#[route(GET "/manifest.webmanifest")]
async fn manifest(cx: &Cx) -> Result<Response> {
    let assets = asset_config(cx);
    let body = format!(
        r##"{{
  "name": "Food",
  "short_name": "Food",
  "description": "A fast personal food and activity diary",
  "start_url": "/",
  "display": "standalone",
  "background_color": "#f8f7f4",
  "theme_color": "#f8f7f4",
  "icons": [
    {{
      "src": "{icon_192}",
      "sizes": "192x192",
      "type": "image/png",
      "purpose": "any maskable"
    }},
    {{
      "src": "{icon_512}",
      "sizes": "512x512",
      "type": "image/png",
      "purpose": "any maskable"
    }}
  ]
}}"##,
        icon_192 = assets.resolve(APP_ICON_192),
        icon_512 = assets.resolve(APP_ICON_512),
    );

    (
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/manifest+json"),
            ),
            (
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=3600"),
            ),
        ],
        body,
    )
        .into_response(cx)
}

#[route(GET "/service-worker.js")]
async fn service_worker(cx: &Cx) -> Result<Response> {
    let assets = asset_config(cx);
    let body = format!(
        r#"importScripts("{core}", "{routing}", "{strategies}");

self.skipWaiting();
workbox.core.clientsClaim();

workbox.routing.registerRoute(
    ({{request}}) => request.mode === "navigate",
    new workbox.strategies.NetworkOnly()
);

workbox.routing.registerRoute(
    ({{request, url}}) => request.method === "GET" && url.origin === self.location.origin &&
        (url.pathname.startsWith("/_topcoat/assets/") || url.pathname.startsWith("/_topcoat/fonts/")),
    new workbox.strategies.CacheFirst({{cacheName: "food-public-assets-v1"}})
);
"#,
        core = assets.resolve(WORKBOX_CORE),
        routing = assets.resolve(WORKBOX_ROUTING),
        strategies = assets.resolve(WORKBOX_STRATEGIES),
    );

    (
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/javascript; charset=utf-8"),
            ),
            (header::CACHE_CONTROL, HeaderValue::from_static("no-cache")),
        ],
        body,
    )
        .into_response(cx)
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
