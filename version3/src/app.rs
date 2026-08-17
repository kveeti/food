use topcoat::{
    Result,
    asset::{Asset, asset},
    context::Cx,
    font::{Font, font},
    router::{RouterBuilder, error::RouterErrorExt, layout, page},
    tailwind,
    view::view,
};

use crate::{
    auth,
    day::{self, day_navigation},
    db,
    pwa::APP_ICON_192,
    settings::SETTINGS_JS,
    water::{self, water_section},
};

const APP_ICON: Asset = asset!("public/app-icon.svg");
pub const GEIST: Font = font! {
    "Geist",
    @font-face {
        src: url(asset!("public/fonts/geist.woff2")) format("woff2");
        font-style: normal;
        font-weight: 100 900;
        font-display: swap;
    }
};

pub fn register(builder: RouterBuilder) -> RouterBuilder {
    builder.layout(root_layout).page(home)
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
                <script defer="true" src=(SETTINGS_JS)></script>
                <script defer="true" src=(asset!("public/register-service-worker.js"))></script>
            </head>

            <body class="min-h-screen bg-canvas font-sans text-gray-950 antialiased">
                <nav class="fixed inset-x-0 bottom-0 z-10 h-(--nav-height) bg-nav/80 backdrop-blur-md sm:sticky sm:top-0">
                    <div class="mx-auto flex h-full w-full max-w-(--page-width) items-stretch justify-between px-3 sm:px-6">
                        <div class="flex h-full items-stretch">
                            <a
                                href="/"
                                hx-boost="true"
                                class="inline-flex h-full items-center px-3 text-sm text-gray-950 hover:bg-gray-200/70 sm:text-base"
                            >
                                "today"
                            </a>
                            <a
                                href="/settings"
                                hx-boost="true"
                                class="inline-flex h-full items-center px-3 text-sm text-gray-700 hover:bg-gray-200/70 sm:text-base"
                            >
                                "settings"
                            </a>
                        </div>
                        <form method="post" action="/logout">
                            <button type="submit" class="inline-flex h-full items-center px-3 text-sm text-gray-700 hover:bg-gray-200/70 sm:text-base">
                                "log out"
                            </button>
                        </form>
                    </div>
                </nav>

                (slot?)
            </body>
        </html>
    }
}

#[page("/")]
async fn home(cx: &Cx) -> Result {
    let user = auth::require_user(cx).await?;
    let timezone = user.timezone.ok_or_redirect("/settings?required=1")?;
    let day = day::Day::from_request(cx, db(cx), &timezone).await?;
    let water = water::load(db(cx), user.id, day.date, &timezone).await?;

    view! {
        <main class="mx-auto w-full max-w-(--page-width) px-3 pb-[calc(var(--nav-height)+2rem)] pt-6 sm:px-6 sm:pb-12 sm:pt-10">
            day_navigation(day: &day)

            <div class="min-h-32"></div>

            water_section(day: &day, water: &water)
        </main>
    }
}
