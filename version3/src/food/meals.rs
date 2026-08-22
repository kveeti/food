use super::*;
use crate::components::{button, input};

pub(super) fn register(builder: RouterBuilder) -> RouterBuilder {
    builder.route(preview_meal).route(copy_meal)
}

pub(super) async fn load_latest_meal(
    pool: &PgPool,
    user_id: Uuid,
    date: NaiveDate,
    timezone: &str,
) -> Result<Option<LatestMeal>> {
    let row: Option<(Uuid, Option<String>, String, bool)> = sqlx::query_as(
        "WITH target AS (
             SELECT (($2::date + (now() AT TIME ZONE $3)::time)::timestamp AT TIME ZONE $3) AS eaten_at
         )
         SELECT meals.id, meals.name,
                to_char(meals.started_at AT TIME ZONE $3, 'HH24:MI'),
                MAX(entries.eaten_at) BETWEEN target.eaten_at - INTERVAL '2 hours' AND target.eaten_at
         FROM meals
         JOIN food_entries entries ON entries.meal_id = meals.id
         CROSS JOIN target
         WHERE meals.user_id = $1
           AND entries.user_id = $1
           AND entries.eaten_at >= ($2::date::timestamp AT TIME ZONE $3)
           AND entries.eaten_at < (($2::date + 1)::timestamp AT TIME ZONE $3)
         GROUP BY meals.id, target.eaten_at
         ORDER BY MAX(entries.eaten_at) DESC
         LIMIT 1",
    )
    .bind(user_id)
    .bind(date)
    .bind(timezone)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(id, name, local_time, active)| LatestMeal {
        id,
        name,
        local_time,
        active,
    }))
}

pub(super) async fn load_meal_preview(
    pool: &PgPool,
    user_id: Uuid,
    meal_id: Uuid,
    date: NaiveDate,
) -> Result<MealPreview> {
    type Row = (
        Option<String>,
        Uuid,
        String,
        Option<String>,
        f64,
        String,
        bool,
    );
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT meals.name,
                entries.id, entries.food_name, entries.food_brand, entries.amount, entries.unit,
                current_food.id IS NOT NULL
         FROM meals
         JOIN food_entries entries ON entries.meal_id = meals.id AND entries.user_id = $1
         LEFT JOIN foods current_food
                ON current_food.id = entries.food_id
               AND NOT current_food.is_archived
               AND (current_food.source <> 'custom' OR current_food.owner_user_id = $1)
         WHERE meals.id = $2 AND meals.user_id = $1
         ORDER BY entries.eaten_at, entries.created_at",
    )
    .bind(user_id)
    .bind(meal_id)
    .fetch_all(pool)
    .await?;
    let Some(first) = rows.first() else {
        return Err(not_found().into());
    };

    Ok(MealPreview {
        id: meal_id,
        date,
        name: first.0.clone(),
        entries: rows
            .into_iter()
            .map(|row| MealPreviewEntry {
                id: row.1,
                name: row.2,
                brand: row.3,
                amount: row.4,
                unit: row.5,
                available: row.6,
            })
            .collect(),
    })
}

#[path_param(error = bad_request)]
struct MealId(Uuid);

#[topcoat::router::query_params(error = bad_request)]
struct MealPreviewQuery {
    date: Option<String>,
}

#[route(GET "/meals/{meal_id}/preview")]
async fn preview_meal(cx: &Cx) -> Result<Response> {
    let user = auth::require_user(cx).await?;
    user.timezone.ok_or_redirect("/settings?required=1")?;
    let meal = path_param::<MealId>(cx)?;
    let query = query_params::<MealPreviewQuery>(cx)?;
    let date = query
        .date
        .as_deref()
        .map(parse_date)
        .transpose()?
        .ok_or_else(|| bad_request("date is required"))?;
    let preview = load_meal_preview(db(cx), user.id, *meal, date).await?;
    let fragment = view! {
        meal_preview(preview: &preview)
    }?;
    fragment.into_response(cx)
}

#[route(POST "/meals/{meal_id}/copy")]
async fn copy_meal(cx: &Cx, Form(input): Form<HashMap<String, String>>) -> Result<Response> {
    let user = auth::require_user(cx).await?;
    let timezone = user.timezone.ok_or_redirect("/settings?required=1")?;
    let source_meal_id = path_param::<MealId>(cx)?;
    let date = input
        .get("date")
        .map(String::as_str)
        .map(parse_date)
        .transpose()?
        .ok_or_else(|| bad_request("date is required"))?;
    let meal_name = parse_meal_name(input.get("meal_name").map(String::as_str))?;

    let mut entries = Vec::new();
    for key in input.keys() {
        let Some(id) = key.strip_prefix("include_") else {
            continue;
        };
        let id = Uuid::parse_str(id).map_err(|_| bad_request("invalid meal entry"))?;
        let amount = input
            .get(&format!("amount_{id}"))
            .ok_or_else(|| bad_request("amount is required"))?;
        entries.push((id, parse_required_amount(amount)?));
    }
    if entries.is_empty() {
        return Err(bad_request("select at least one food").into());
    }
    entries.sort_unstable_by_key(|entry| entry.0);
    let entry_ids: Vec<Uuid> = entries.iter().map(|entry| entry.0).collect();
    let amounts: Vec<f64> = entries.iter().map(|entry| entry.1).collect();

    let mut transaction = db(cx).begin().await?;
    let eaten_at: DateTime<Utc> = sqlx::query_scalar(
        "SELECT (($1::date + (now() AT TIME ZONE $2)::time)::timestamp AT TIME ZONE $2)",
    )
    .bind(date)
    .bind(&timezone)
    .fetch_one(&mut *transaction)
    .await?;
    let new_meal_id: Option<Uuid> = sqlx::query_scalar(
        "INSERT INTO meals (user_id, name, started_at)
         SELECT $1, $3, $4
         FROM meals
         WHERE id = $2 AND user_id = $1
         RETURNING id",
    )
    .bind(user.id)
    .bind(*source_meal_id)
    .bind(meal_name)
    .bind(eaten_at)
    .fetch_optional(&mut *transaction)
    .await?;
    let Some(new_meal_id) = new_meal_id else {
        return Err(not_found().into());
    };

    let copied: i64 = sqlx::query_scalar(
        "WITH input AS (
             SELECT *
             FROM unnest($4::uuid[], $5::double precision[]) AS selected(entry_id, amount)
         ), selected_entries AS (
             SELECT source_entries.id, foods.id AS food_id, input.amount,
                    foods.basis_unit, foods.display_name, foods.brand,
                    foods.source, foods.source_id,
                    source_entries.eaten_at, source_entries.created_at
             FROM input
             JOIN food_entries source_entries
                  ON source_entries.id = input.entry_id
                 AND source_entries.meal_id = $1
                 AND source_entries.user_id = $2
             JOIN foods
                  ON foods.id = source_entries.food_id
                 AND NOT foods.is_archived
                 AND (foods.source <> 'custom' OR foods.owner_user_id = $2)
         ), new_entries AS (
             INSERT INTO food_entries (
                 user_id, meal_id, food_id, amount, unit, eaten_at,
                 food_name, food_brand, food_source, food_source_id
             )
             SELECT $2, $3, food_id, amount, basis_unit, $6,
                    display_name, brand, source, source_id
             FROM selected_entries
             ORDER BY eaten_at, created_at
             RETURNING id, food_id, amount, unit
         ), nutrient_snapshot AS (
             INSERT INTO food_entry_nutrients (
                 food_entry_id, nutrient_id, basis_value, consumed_value
             )
             SELECT new_entries.id, food_nutrients.nutrient_id, food_nutrients.value,
                    CASE new_entries.unit
                        WHEN 'count' THEN food_nutrients.value * new_entries.amount
                        ELSE food_nutrients.value * new_entries.amount / 100.0
                    END
             FROM new_entries
             JOIN food_nutrients ON food_nutrients.food_id = new_entries.food_id
         )
         SELECT COUNT(*) FROM new_entries",
    )
    .bind(*source_meal_id)
    .bind(user.id)
    .bind(new_meal_id)
    .bind(&entry_ids)
    .bind(&amounts)
    .bind(eaten_at)
    .fetch_one(&mut *transaction)
    .await?;
    if copied != entry_ids.len() as i64 {
        return Err(bad_request("a selected food is no longer available").into());
    }
    transaction.commit().await?;

    redirect_to_day(cx, date)
}

pub(super) fn meal_url(meal_id: Uuid, date: NaiveDate) -> String {
    format!("/?date={date}&meal_preview={meal_id}#food-preview")
}

pub(super) fn meal_preview_url(meal_id: Uuid, date: NaiveDate) -> String {
    format!("/meals/{meal_id}/preview?date={date}")
}

fn cancel_meal_copy_url(date: NaiveDate) -> String {
    format!("/?date={date}#food-section")
}

#[topcoat::view::component]
pub(super) async fn meal_preview(preview: &MealPreview) -> Result {
    view! {
        <div id="food-preview" class="mt-5 scroll-mt-4 overflow-hidden rounded-xl border border-gray-200 sm:scroll-mt-[calc(var(--nav-height)+1rem)]">
            <div class="px-4 pb-3 pt-4">
                <h3 class="text-base font-semibold text-gray-1000">"New meal"</h3>
            </div>

            <form method="post" action=(format!("/meals/{}/copy", preview.id)) data-meal-stage="true">
                <input type="hidden" name="date" value=(preview.date.to_string())>
                <label class="mb-3 block px-4 text-sm text-gray-700">
                    <span class="mb-1 block">"Meal name"</span>
                    <input
                        name="meal_name"
                        list="copy-meal-name-suggestions"
                        maxlength="100"
                        autocomplete="off"
                        value=(preview.name.as_deref().unwrap_or_default())
                        class=(input::FIELD)
                    >
                </label>
                <datalist id="copy-meal-name-suggestions">
                    <option value="Breakfast"></option>
                    <option value="Lunch"></option>
                    <option value="Dinner"></option>
                    <option value="Snack"></option>
                </datalist>
                <div class="mb-1 flex justify-start">
                    <label data-select-all-control="true" hidden="true" class="flex w-full cursor-pointer items-center gap-3 px-4 py-1 text-xs font-medium text-gray-700">
                        <input type="checkbox" data-select-all="true" class="size-4 accent-gray-800">
                        <span>"Select all"</span>
                    </label>
                </div>
                <ul class="border-y border-gray-200">
                    for entry in &preview.entries {
                        <li class="flex min-h-12 items-stretch justify-between border-t border-gray-200 first:border-t-0">
                            <label class="flex min-w-0 flex-1 cursor-pointer items-center gap-3 py-2 pl-4 pr-4">
                                <input
                                    name=(format!("include_{}", entry.id))
                                    type="checkbox"
                                    data-meal-entry="true"
                                    checked=(entry.available)
                                    disabled=(!entry.available)
                                    class="size-4 shrink-0 accent-gray-800"
                                >
                                <span class="min-w-0">
                                    <span class="block truncate text-sm text-gray-900">(&entry.name)</span>
                                    if let Some(brand) = &entry.brand {
                                        <span class="block truncate text-xs text-gray-500">(brand)</span>
                                    }
                                    if !entry.available {
                                        <span class="block text-xs text-danger-text">"Unavailable"</span>
                                    }
                                </span>
                            </label>
                            <span class="flex shrink-0 items-center gap-2 py-2 pr-4 text-sm text-gray-600">
                                <input
                                    name=(format!("amount_{}", entry.id))
                                    type="number"
                                    inputmode="decimal"
                                    min="0"
                                    max="100000"
                                    step="any"
                                    required="true"
                                    disabled=(!entry.available)
                                    value=(entry.amount)
                                    aria-label=(format!("Amount for {} in {}", entry.name, unit_name(&entry.unit)))
                                    class="w-24 rounded-lg border border-gray-300 bg-form px-2 py-1.5 text-right text-base tabular-nums text-gray-1000 outline-none disabled:opacity-50 focus:border-gray-500 focus:ring-1 focus:ring-gray-400"
                                >
                                <span>(entry_unit(&entry.unit))</span>
                            </span>
                        </li>
                    }
                </ul>

                <div class="mx-4 mb-4 mt-4 flex gap-4">
                    <a
                        href=(cancel_meal_copy_url(preview.date))
                        hx-boost="true"
                        class=(button::GHOST)
                    >
                        "Cancel"
                    </a>
                    <button type="submit" class=(format!("flex-1 {}", button::OUTLINE))>
                        "Add meal"
                    </button>
                </div>
            </form>
        </div>
    }
}
