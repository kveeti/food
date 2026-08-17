use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{PgPool, types::Uuid};
use topcoat::{
    Result,
    context::Cx,
    htmx::hx_request,
    router::{
        HeaderValue, IntoResponse, Response, RouterBuilder, StatusCode,
        content::Form,
        error::{RouterErrorExt, bad_request},
        header, path_param, route,
    },
    view::view,
};

use crate::{auth, day::Day, db};

pub fn register(builder: RouterBuilder) -> RouterBuilder {
    builder.route(log_water).route(delete_water)
}

#[derive(Debug, serde::Deserialize)]
struct WaterForm {
    amount_ml: i32,
    date: String,
}

#[derive(Debug)]
struct WaterEntry {
    id: Uuid,
    amount_ml: i32,
    consumed_at: DateTime<Utc>,
    local_time: String,
}

#[derive(Debug)]
pub struct WaterDay {
    entries: Vec<WaterEntry>,
    total: i64,
}

pub async fn load(
    pool: &PgPool,
    user_id: Uuid,
    date: NaiveDate,
    timezone: &str,
) -> Result<WaterDay> {
    let (rows, (_, total)) = tokio::try_join!(
        sqlx::query_as(
            "SELECT id, amount_ml, consumed_at,
                    to_char(consumed_at AT TIME ZONE $3, 'HH24:MI')
             FROM water_log
             WHERE user_id = $1
               AND consumed_at >= ($2::date::timestamp AT TIME ZONE $3)
               AND consumed_at < (($2::date + 1)::timestamp AT TIME ZONE $3)
             ORDER BY consumed_at DESC",
        )
        .bind(user_id)
        .bind(date)
        .bind(timezone)
        .fetch_all(pool),
        load_summary(pool, user_id, date, timezone),
    )?;
    let entries = rows
        .into_iter()
        .map(|(id, amount_ml, consumed_at, local_time)| WaterEntry {
            id,
            amount_ml,
            consumed_at,
            local_time,
        })
        .collect();

    Ok(WaterDay { entries, total })
}

async fn load_summary(
    pool: &PgPool,
    user_id: Uuid,
    date: NaiveDate,
    timezone: &str,
) -> sqlx::Result<(i64, i64)> {
    sqlx::query_as(
        "SELECT COUNT(*), COALESCE(SUM(amount_ml), 0)
         FROM water_log
         WHERE user_id = $1
           AND consumed_at >= ($2::date::timestamp AT TIME ZONE $3)
           AND consumed_at < (($2::date + 1)::timestamp AT TIME ZONE $3)",
    )
    .bind(user_id)
    .bind(date)
    .bind(timezone)
    .fetch_one(pool)
    .await
}

#[topcoat::view::component]
pub async fn water_section(day: &Day, water: &WaterDay) -> Result {
    let date = day.date.to_string();

    view! {
        <section id="water-section" aria-labelledby="water-heading" class="mt-14 border-t border-gray-200 pt-5">
            <div class="mb-3 flex items-baseline justify-between gap-4">
                <h2 id="water-heading" class="text-base font-medium text-gray-1000">"Water"</h2>
                <p aria-label="Water total" class="text-sm tabular-nums text-gray-600">
                    <span id="water-total-value" class="font-medium text-gray-900">(water.total) " ml"</span>
                </p>
            </div>

            <div class="grid grid-cols-2 gap-2">
                water_button(amount_ml: 250, date: day.date)
                water_button(amount_ml: 750, date: day.date)
            </div>

            <details class="mt-2 border-b border-gray-200 pb-2">
                <summary class="cursor-pointer py-2 text-sm text-gray-600 hover:text-gray-900">"Other amount"</summary>
                <form
                    method="post"
                    action="/water"
                    class="flex gap-2 pb-2"
                    hx-post="/water"
                    hx-target="#water-entries"
                    hx-swap="afterbegin"
                    hx-optimistic="#water-optimistic"
                >
                    <input type="hidden" name="date" value=(&date)>
                    <label for="custom-water" class="sr-only">"Water in millilitres"</label>
                    <input
                        id="custom-water"
                        name="amount_ml"
                        type="number"
                        inputmode="numeric"
                        min="1"
                        max="10000"
                        required="true"
                        placeholder="Millilitres"
                        class="min-w-0 flex-1 rounded-lg border border-gray-300 bg-form px-3 py-2 text-base text-gray-1000 outline-none placeholder:text-gray-500 hover:border-gray-400 focus:border-gray-500 focus:ring-1 focus:ring-gray-400"
                    >
                    <button
                        type="submit"
                        class="rounded-lg bg-gray-900 px-4 py-2 text-base font-medium text-white hover:bg-gray-800 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-gray-700 active:bg-gray-1000"
                    >
                        "Add"
                    </button>
                </form>
            </details>

            <template id="water-optimistic">
                <li class="flex min-h-9 items-center justify-between border-t border-gray-200 py-1.5 text-sm opacity-50 first:border-t-0">
                    <span class="optimistic-amount font-medium text-gray-900"></span>
                    <span class="text-gray-500">"adding..."</span>
                </li>
            </template>

            water_history(entries: &water.entries, date: day.date, open: day.water_open)
        </section>
    }
}

#[topcoat::view::component]
async fn water_button(amount_ml: i32, date: NaiveDate) -> Result {
    let date = date.to_string();

    view! {
        <form
            method="post"
            action="/water"
            hx-post="/water"
            hx-target="#water-entries"
            hx-swap="afterbegin"
            hx-optimistic="#water-optimistic"
        >
            <input type="hidden" name="amount_ml" value=(amount_ml)>
            <input type="hidden" name="date" value=(date)>
            <button
                type="submit"
                class="w-full rounded-lg bg-gray-900 px-3 py-2 text-sm font-medium text-white hover:bg-gray-800 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-gray-700 active:bg-gray-1000"
            >
                (amount_ml) " ml"
            </button>
        </form>
    }
}

#[route(POST "/water")]
async fn log_water(cx: &Cx, Form(input): Form<WaterForm>) -> Result<Response> {
    let user = auth::require_user(cx).await?;
    let timezone = user.timezone.ok_or_redirect("/settings?required=1")?;
    let date = parse_date(&input.date)?;
    if !(1..=10_000).contains(&input.amount_ml) {
        return Err(bad_request("water must be between 1 and 10000 ml").into());
    }

    let (id, amount_ml, consumed_at, local_time): (Uuid, i32, DateTime<Utc>, String) =
        sqlx::query_as(
            "INSERT INTO water_log (user_id, amount_ml, consumed_at)
             VALUES (
                 $1,
                 $2,
                 (($3::date + (now() AT TIME ZONE $4)::time)::timestamp AT TIME ZONE $4)
             )
             RETURNING id, amount_ml, consumed_at,
                 to_char(consumed_at AT TIME ZONE $4, 'HH24:MI')",
        )
        .bind(user.id)
        .bind(input.amount_ml)
        .bind(date)
        .bind(&timezone)
        .fetch_one(db(cx))
        .await?;
    let entry = WaterEntry {
        id,
        amount_ml,
        consumed_at,
        local_time,
    };

    if !hx_request(cx) {
        return redirect_to_day(cx, date, false);
    }

    let (count, total) = load_summary(db(cx), user.id, date, &timezone).await?;
    let count_label = entry_count(count as usize);
    let fragment = view! {
        water_entry(entry: &entry, date: date)
        <span id="water-total-value" hx-swap-oob="outerHTML" class="font-medium text-gray-900">(total) " ml"</span>
        <span id="water-entry-count" hx-swap-oob="outerHTML">(count_label)</span>
        <li id="water-empty-state" hx-swap-oob="delete"></li>
    }?;

    fragment.into_response(cx)
}

#[path_param(error = bad_request)]
struct Id(Uuid);

#[derive(Debug, serde::Deserialize)]
struct DeleteForm {
    date: String,
}

#[route(POST "/water/{id}/delete")]
async fn delete_water(cx: &Cx, Form(input): Form<DeleteForm>) -> Result<Response> {
    let user = auth::require_user(cx).await?;
    let timezone = user.timezone.ok_or_redirect("/settings?required=1")?;
    let date = parse_date(&input.date)?;
    let id = path_param::<Id>(cx)?;

    sqlx::query("DELETE FROM water_log WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user.id)
        .execute(db(cx))
        .await?;

    if !hx_request(cx) {
        return redirect_to_day(cx, date, true);
    }

    let water = load(db(cx), user.id, date, &timezone).await?;
    let fragment = view! {
        water_history(entries: &water.entries, date: date, open: true)
        <span id="water-total-value" hx-swap-oob="outerHTML" class="font-medium text-gray-900">(water.total) " ml"</span>
    }?;

    fragment.into_response(cx)
}

fn redirect_to_day(cx: &Cx, date: NaiveDate, water_open: bool) -> Result<Response> {
    let location = if water_open {
        format!("/?date={date}&water=open")
    } else {
        format!("/?date={date}")
    };
    (
        StatusCode::SEE_OTHER,
        [(header::LOCATION, HeaderValue::from_str(&location)?)],
        (),
    )
        .into_response(cx)
}

fn parse_date(date: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| bad_request("date must use YYYY-MM-DD").into())
}

fn entry_count(count: usize) -> String {
    match count {
        1 => "1 entry".to_owned(),
        count => format!("{count} entries"),
    }
}

#[topcoat::view::component]
async fn water_history(entries: &[WaterEntry], date: NaiveDate, open: bool) -> Result {
    let count = entry_count(entries.len());

    view! {
        <details id="water-history" open=(open) class="mt-1">
            <summary class="cursor-pointer py-2 text-sm text-gray-600 hover:text-gray-900">
                <span id="water-entry-count">(count)</span>
            </summary>
            <ul id="water-entries" class="border-y border-gray-200">
                if entries.is_empty() {
                    <li id="water-empty-state" class="py-4 text-center text-sm text-gray-500">"No water logged"</li>
                } else {
                    for entry in entries {
                        water_entry(entry: entry, date: date)
                    }
                }
            </ul>
        </details>
    }
}

#[topcoat::view::component]
async fn water_entry(entry: &WaterEntry, date: NaiveDate) -> Result {
    let date = date.to_string();

    view! {
        <li class="border-t border-gray-200 first:border-t-0">
            <form
                method="post"
                action=(format!("/water/{}/delete", entry.id))
                class="flex min-h-10 items-center justify-between"
                hx-post=(format!("/water/{}/delete", entry.id))
                hx-target="#water-history"
                hx-swap="outerHTML"
            >
                <input type="hidden" name="date" value=(date)>
                <span class="text-sm font-medium tabular-nums text-gray-900">(entry.amount_ml) " ml"</span>
                <span class="flex items-center gap-2">
                    <time class="text-sm tabular-nums text-gray-500" datetime=(entry.consumed_at.to_rfc3339())>
                        (&entry.local_time)
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
