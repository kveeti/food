use chrono::{NaiveDate, Utc};
use chrono_tz::Tz;
use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, app_context},
    htmx::hx_request,
    router::{
        RouterBuilder,
        content::Form,
        error::{bad_request, redirect, see_other},
        response::{IntoResponse, Response},
        route,
    },
    view::{attributes, component, view},
};

use crate::{
    auth,
    components::{
        button::{ButtonVariant, button},
        input::input,
    },
    data::Data,
    day::Day,
    settings,
};

pub fn register(builder: RouterBuilder) -> RouterBuilder {
    builder.route(add_water)
}

#[derive(Deserialize)]
struct WaterForm {
    amount_ml: i32,
    date: String,
}

#[route(POST "/water")]
#[tracing::instrument(name = "water::add_water", level = "debug", skip_all)]
async fn add_water(cx: &Cx, Form(form): Form<WaterForm>) -> Result<Response> {
    let user = auth::require_user(cx).await?;
    let Some(timezone_name) = user.timezone.as_deref() else {
        return Err(redirect(settings::setup_url("/")).into());
    };
    let timezone = timezone_name
        .parse::<Tz>()
        .map_err(|_| bad_request("invalid user timezone"))?;
    let date = NaiveDate::parse_from_str(&form.date, "%Y-%m-%d")
        .map_err(|_| bad_request("date must use YYYY-MM-DD"))?;
    if !(10..=1_500).contains(&form.amount_ml) {
        return Err(bad_request("water must be between 10 and 1500 ml").into());
    }

    let total = data(cx)
        .add_water(user.id, form.amount_ml, date, timezone_name)
        .await?;
    if hx_request(cx) {
        return view! { water_total(total: total) }?.into_response(cx);
    }

    let day = Day {
        date,
        today: Utc::now().with_timezone(&timezone).date_naive(),
    };
    see_other(day.url()).into_response(cx)
}

#[component]
async fn water_total(total: i64) -> Result {
    view! {
        <p
            id="water-total"
            aria-label="Water total"
            class="text-2xl font-semibold tabular-nums"
        >
            (total)
            " ml"
        </p>
    }
}

#[component]
pub async fn water_section(day: Day, total: i64) -> Result {
    let date = day.date.to_string();

    view! {
        <section
            id="water-section"
            aria-labelledby="water-heading"
            class="mt-10 border-t border-gray-200 pt-6 dark:border-gray-800"
        >
            <header class="flex justify-end">
                <h2 id="water-heading" class="sr-only">"Water"</h2>
                water_total(total: total)
            </header>

            <form
                method="post"
                action="/water"
                hx-post="/water"
                hx-target="#water-total"
                hx-swap="outerHTML"
                class="mt-6 flex flex-col items-center"
                data-water-glass="true"
            >
                <input
                    disabled="disabled"
                    data-water-amount="true"
                    type="hidden"
                    name="amount_ml"
                    value="250"
                >
                <noscript>
                    <style>
                        "[data-water-stage], [data-water-output] { display: none; }"
                    </style>
                    <div class="w-full max-w-xs">
                        input(
                            label: Some("Amount (ml)"),
                            attrs: attributes! {
                                type="number"
                                name="amount_ml"
                                min="10"
                                max="1500"
                                step="10"
                                inputmode="numeric"
                                value="250"
                            }
                        )
                    </div>
                </noscript>
                <input type="hidden" name="date" value=(&date)>
                <div data-water-stage="true" class="invisible relative h-[420px] w-48">
                    <div
                        data-water-vessel="glass"
                        data-water-min="10"
                        data-water-max="600"
                        data-water-default="250"
                        data-water-top="10"
                        data-water-bottom="220"
                        data-selected="true"
                        role="slider"
                        tabindex="0"
                        aria-label="Glass amount"
                        aria-valuemin="10"
                        aria-valuemax="600"
                        aria-valuenow="250"
                        class="water-vessel h-[280px] w-[180px] rounded-2xl outline-2 outline-transparent outline-offset-4 focus-visible:outline-outline"
                    >
                        <svg
                            viewBox="0 0 140 230"
                            preserveAspectRatio="none"
                            class="h-full w-full"
                            aria-hidden="true"
                        >
                            <defs>
                                <clipPath id="glass-water-clip">
                                    <path
                                        d="M10 10 H130 L115 207 Q114 220 100 220 H40 Q26 220 25 207 Z"
                                    ></path>
                                </clipPath>
                                <clipPath id="glass-bubble-clip">
                                    <rect
                                        data-water-bubble-clip="glass"
                                        class="water-bubble-clip"
                                        x="0"
                                        y="0"
                                        width="140"
                                        height="300"
                                        style="transform: translateY(129.44px)"
                                    ></rect>
                                </clipPath>
                            </defs>
                            <path
                                class="water-vessel-outline"
                                d="M10 10 H130 L115 207 Q114 220 100 220 H40 Q26 220 25 207 Z"
                            ></path>
                            <g clip-path="url(#glass-water-clip)">
                                <g
                                    data-water-liquid="glass"
                                    class="water-liquid"
                                    style="transform: translateY(129.44px)"
                                >
                                    <path
                                        class="water-wave water-wave-back"
                                        d="M-192 4 C-168 2 -120 2 -96 4 C-72 6 -24 6 0 4 C24 2 72 2 96 4 C120 6 168 6 192 4 C216 2 264 2 288 4 C312 6 360 6 384 4 V300 H-192 Z"
                                    ></path>
                                    <path
                                        class="water-wave water-wave-front"
                                        d="M-192 5 C-168 2 -120 2 -96 5 C-72 8 -24 8 0 5 C24 2 72 2 96 5 C120 8 168 8 192 5 C216 2 264 2 288 5 C312 8 360 8 384 5 V300 H-192 Z"
                                    ></path>
                                </g>
                                <g clip-path="url(#glass-bubble-clip)">
                                    <g class="water-bubbles">
                                        <g class="water-bubble-path water-bubble-one">
                                            <circle class="water-bubble" cx="35" cy="0" r="2.5"></circle>
                                        </g>
                                        <g class="water-bubble-path water-bubble-two">
                                            <circle class="water-bubble" cx="66" cy="0" r="2"></circle>
                                        </g>
                                        <g class="water-bubble-path water-bubble-three">
                                            <circle class="water-bubble" cx="98" cy="0" r="3"></circle>
                                        </g>
                                        <g class="water-bubble-path water-bubble-four">
                                            <circle class="water-bubble" cx="51" cy="0" r="2.5"></circle>
                                        </g>
                                        <g class="water-bubble-path water-bubble-five">
                                            <circle class="water-bubble" cx="84" cy="0" r="1.75"></circle>
                                        </g>
                                    </g>
                                </g>
                            </g>
                        </svg>
                    </div>
                    <div
                        data-water-vessel="bottle"
                        data-water-min="100"
                        data-water-max="1500"
                        data-water-default="1000"
                        data-water-top="24"
                        data-water-bottom="232"
                        data-selected="false"
                        role="slider"
                        tabindex="0"
                        aria-label="Bottle amount"
                        aria-valuemin="100"
                        aria-valuemax="1500"
                        aria-valuenow="1000"
                        class="water-vessel h-[310px] w-[190px] rounded-2xl outline-2 outline-transparent outline-offset-4 focus-visible:outline-outline"
                    >
                        <svg
                            viewBox="0 0 140 240"
                            preserveAspectRatio="none"
                            class="h-full w-full"
                            aria-hidden="true"
                        >
                            <defs>
                                <clipPath id="bottle-water-clip">
                                    <path
                                        d="M58 6 H82 Q88 6 88 12 V18 Q88 22 86 22 Q88 24 92 24 C114 24 132 46 132 72 V204 Q132 230 106 230 H34 Q8 230 8 204 V72 C8 46 26 24 48 24 Q52 24 54 22 Q52 22 52 18 V12 Q52 6 58 6 Z"
                                    ></path>
                                </clipPath>
                                <clipPath id="bottle-bubble-clip">
                                    <rect
                                        data-water-bubble-clip="bottle"
                                        class="water-bubble-clip"
                                        x="0"
                                        y="0"
                                        width="140"
                                        height="300"
                                        style="transform: translateY(91.6px)"
                                    ></rect>
                                </clipPath>
                            </defs>
                            <path
                                class="water-vessel-outline"
                                d="M58 6 H82 Q88 6 88 12 V18 Q88 22 86 22 Q88 24 92 24 C114 24 132 46 132 72 V204 Q132 230 106 230 H34 Q8 230 8 204 V72 C8 46 26 24 48 24 Q52 24 54 22 Q52 22 52 18 V12 Q52 6 58 6 Z"
                            ></path>
                            <g clip-path="url(#bottle-water-clip)">
                                <g
                                    data-water-liquid="bottle"
                                    class="water-liquid"
                                    style="transform: translateY(91.6px)"
                                >
                                    <path
                                        class="water-wave water-wave-back"
                                        d="M-192 4 C-168 2 -120 2 -96 4 C-72 6 -24 6 0 4 C24 2 72 2 96 4 C120 6 168 6 192 4 C216 2 264 2 288 4 C312 6 360 6 384 4 V300 H-192 Z"
                                    ></path>
                                    <path
                                        class="water-wave water-wave-front"
                                        d="M-192 5 C-168 2 -120 2 -96 5 C-72 8 -24 8 0 5 C24 2 72 2 96 5 C120 8 168 8 192 5 C216 2 264 2 288 5 C312 8 360 8 384 5 V300 H-192 Z"
                                    ></path>
                                </g>
                                <g clip-path="url(#bottle-bubble-clip)">
                                    <g class="water-bubbles">
                                        <g class="water-bubble-path water-bubble-one">
                                            <circle class="water-bubble" cx="35" cy="0" r="2.5"></circle>
                                        </g>
                                        <g class="water-bubble-path water-bubble-two">
                                            <circle class="water-bubble" cx="66" cy="0" r="2"></circle>
                                        </g>
                                        <g class="water-bubble-path water-bubble-three">
                                            <circle class="water-bubble" cx="98" cy="0" r="3"></circle>
                                        </g>
                                        <g class="water-bubble-path water-bubble-four">
                                            <circle class="water-bubble" cx="51" cy="0" r="2.5"></circle>
                                        </g>
                                        <g class="water-bubble-path water-bubble-five">
                                            <circle class="water-bubble" cx="84" cy="0" r="1.75"></circle>
                                        </g>
                                    </g>
                                </g>
                            </g>
                        </svg>
                    </div>
                </div>
                <output
                    data-water-output="true"
                    class="invisible mt-3 text-4xl font-semibold tabular-nums"
                >
                    "250 ml"
                </output>
                <div class="mt-5 w-full max-w-xs">
                    button(
                        variant: ButtonVariant::Accent,
                        attrs: attributes! { class="w-full" },
                        "Add water"
                    )
                </div>
            </form>
        </section>
    }
}

fn data(cx: &Cx) -> &Data {
    app_context(cx)
}
