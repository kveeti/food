use super::*;
use crate::components::{button, input, meal_selector};

pub(super) fn register(builder: RouterBuilder) -> RouterBuilder {
    builder.route(preview_food).route(log_food)
}

pub(super) async fn load_preview(
    cx: &Cx,
    user_id: Uuid,
    food_id: Uuid,
    amount: Option<f64>,
    date: NaiveDate,
    latest_meal: Option<LatestMeal>,
) -> Result<FoodPreview> {
    let rows: Vec<(
        String,
        Option<String>,
        String,
        String,
        String,
        String,
        String,
        f64,
    )> = sqlx::query_as(
        "SELECT foods.display_name, foods.brand, foods.source, foods.basis_unit,
                nutrients.code, nutrients.display_name, nutrients.unit, food_nutrients.value
         FROM foods
         JOIN food_nutrients ON food_nutrients.food_id = foods.id
         JOIN nutrients ON nutrients.id = food_nutrients.nutrient_id
         WHERE foods.id = $1
           AND NOT foods.is_archived
           AND (foods.source <> 'custom' OR foods.owner_user_id = $2)
         ORDER BY nutrients.display_order, nutrients.display_name",
    )
    .bind(food_id)
    .bind(user_id)
    .fetch_all(db(cx))
    .await?;
    let Some(first) = rows.first() else {
        return Err(not_found().into());
    };
    let amount = amount.unwrap_or(if first.3 == "count" { 1.0 } else { 100.0 });
    let nutrients = rows
        .iter()
        .map(|row| NutrientValue {
            code: row.4.clone(),
            name: row.5.clone(),
            unit: row.6.clone(),
            value: row.7,
        })
        .collect();

    Ok(FoodPreview {
        id: food_id,
        name: first.0.clone(),
        brand: first.1.clone(),
        source: first.2.clone(),
        basis_unit: first.3.clone(),
        amount,
        date,
        nutrients,
        latest_meal,
    })
}

#[path_param(error = bad_request)]
struct FoodId(Uuid);

#[topcoat::router::query_params(error = bad_request)]
struct PreviewQuery {
    amount: Option<String>,
    date: Option<String>,
}

#[route(GET "/foods/{food_id}/preview")]
async fn preview_food(cx: &Cx) -> Result<Response> {
    let user = auth::require_user(cx).await?;
    let timezone = user.timezone.ok_or_redirect("/settings?required=1")?;
    let food = path_param::<FoodId>(cx)?;
    let query = query_params::<PreviewQuery>(cx)?;
    let amount = parse_amount(query.amount.as_deref())?;
    let date = query
        .date
        .as_deref()
        .map(parse_date)
        .transpose()?
        .ok_or_else(|| bad_request("date is required"))?;
    let latest_meal = meals::load_latest_meal(db(cx), user.id, date, &timezone).await?;
    let preview = load_preview(cx, user.id, *food, amount, date, latest_meal).await?;
    let fragment = view! {
        food_preview(preview: &preview)
    }?;
    fragment.into_response(cx)
}

#[derive(Debug, serde::Deserialize)]
struct LogFoodForm {
    amount: String,
    date: String,
    meal: String,
    meal_name: Option<String>,
}

#[route(POST "/foods/{food_id}/entries")]
async fn log_food(cx: &Cx, Form(input): Form<LogFoodForm>) -> Result<Response> {
    let user = auth::require_user(cx).await?;
    let timezone = user.timezone.ok_or_redirect("/settings?required=1")?;
    let food_id = path_param::<FoodId>(cx)?;
    let date = parse_date(&input.date)?;
    let amount = parse_required_amount(&input.amount)?;
    let meal_name = parse_meal_name(input.meal_name.as_deref())?;
    let mut transaction = db(cx).begin().await?;
    let eaten_at: DateTime<Utc> = sqlx::query_scalar(
        "SELECT (($1::date + (now() AT TIME ZONE $2)::time)::timestamp AT TIME ZONE $2)",
    )
    .bind(date)
    .bind(&timezone)
    .fetch_one(&mut *transaction)
    .await?;

    let meal_id = if input.meal == "new" {
        sqlx::query_scalar(
            "INSERT INTO meals (user_id, name, started_at)
             VALUES ($1, $2, $3)
             RETURNING id",
        )
        .bind(user.id)
        .bind(meal_name)
        .bind(eaten_at)
        .fetch_one(&mut *transaction)
        .await?
    } else {
        let requested =
            Uuid::parse_str(&input.meal).map_err(|_| bad_request("invalid meal selection"))?;
        let latest: Option<Uuid> = sqlx::query_scalar(
            "SELECT meals.id
             FROM meals
             JOIN food_entries entries ON entries.meal_id = meals.id
             WHERE meals.user_id = $1
               AND entries.user_id = $1
               AND entries.eaten_at >= ($2::date::timestamp AT TIME ZONE $3)
               AND entries.eaten_at < (($2::date + 1)::timestamp AT TIME ZONE $3)
             GROUP BY meals.id
             ORDER BY MAX(entries.eaten_at) DESC
             LIMIT 1",
        )
        .bind(user.id)
        .bind(date)
        .bind(&timezone)
        .fetch_optional(&mut *transaction)
        .await?;
        if latest != Some(requested) {
            return Err(bad_request("only the latest meal can be continued").into());
        }
        requested
    };

    let entry: Option<Uuid> = sqlx::query_scalar(
        "WITH selected_food AS (
             SELECT id, display_name, brand, source, source_id, basis_unit
             FROM foods
             WHERE id = $1
               AND NOT is_archived
               AND (source <> 'custom' OR owner_user_id = $2)
         ), new_entry AS (
             INSERT INTO food_entries (
                 user_id, meal_id, food_id, amount, unit, eaten_at,
                 food_name, food_brand, food_source, food_source_id
             )
             SELECT $2, $3, id, $4, basis_unit, $5,
                    display_name, brand, source, source_id
             FROM selected_food
             RETURNING id, amount, unit
         ), nutrient_snapshot AS (
             INSERT INTO food_entry_nutrients (
                 food_entry_id, nutrient_id, basis_value, consumed_value
             )
             SELECT new_entry.id, food_nutrients.nutrient_id, food_nutrients.value,
                    CASE new_entry.unit
                        WHEN 'count' THEN food_nutrients.value * new_entry.amount
                        ELSE food_nutrients.value * new_entry.amount / 100.0
                    END
             FROM new_entry
             JOIN food_nutrients ON food_nutrients.food_id = $1
         )
         SELECT id FROM new_entry",
    )
    .bind(*food_id)
    .bind(user.id)
    .bind(meal_id)
    .bind(amount)
    .bind(eaten_at)
    .fetch_optional(&mut *transaction)
    .await?;
    if entry.is_none() {
        return Err(not_found().into());
    }
    transaction.commit().await?;

    redirect_to_day(cx, date)
}

pub(super) fn preview_url(food_id: Uuid, date: NaiveDate) -> String {
    format!("/foods/{food_id}/preview?date={date}")
}

#[topcoat::view::component]
pub(super) async fn food_preview(preview: &FoodPreview) -> Result {
    let energy = nutrient_value(preview, "energy").map(|value| value / 4.184);
    let protein = nutrient_value(preview, "protein");
    let carbohydrate = nutrient_value(preview, "carbohydrate");
    let fat = nutrient_value(preview, "fat");
    let fibre = nutrient_value(preview, "fibre");

    view! {
        <div id="food-preview" class="mt-5 scroll-mt-4 rounded-xl border border-gray-200 p-4 sm:scroll-mt-[calc(var(--nav-height)+1rem)]">
            <div class="mb-4">
                <p class="text-xs text-gray-500">(source_name(&preview.source))</p>
                <h3 class="text-base font-semibold text-gray-1000">(&preview.name)</h3>
                if let Some(brand) = &preview.brand {
                    <p class="text-sm text-gray-600">(brand)</p>
                }
            </div>

            <form method="post" action=(format!("/foods/{}/entries", preview.id))>
                <input type="hidden" name="food" value=(preview.id.to_string())>
                <input type="hidden" name="date" value=(preview.date.to_string())>
                <label for="food-amount" class="mb-4 block text-sm text-gray-700">
                    <span class="mb-1 block">"Amount in " (unit_name(&preview.basis_unit))</span>
                    <input
                        id="food-amount"
                        name="amount"
                        type="number"
                        inputmode="decimal"
                        min="0"
                        max="100000"
                        step="any"
                        required="true"
                        value=(preview.amount)
                        class=(input::FIELD)
                    >
                </label>

                <div
                    id="food-nutrition"
                    data-basis-unit=(&preview.basis_unit)
                    data-energy=(data_value(energy))
                    data-protein=(data_value(protein))
                    data-carbohydrate=(data_value(carbohydrate))
                    data-fat=(data_value(fat))
                    data-fibre=(data_value(fibre))
                >
                    <p data-nutrition-label="true" class="mb-2 text-xs font-medium text-gray-600">
                        "Nutrition " (basis_name(&preview.basis_unit))
                    </p>
                    <div class="grid grid-cols-3 gap-2 border-y border-gray-200 py-3">
                        <p>
                            <span class="block text-xs text-gray-500">"Energy"</span>
                            <strong data-nutrition="energy" class="text-sm font-medium tabular-nums">(format_optional(energy)) " kcal"</strong>
                        </p>
                        nutrient_summary(preview: preview, code: "protein", label: "Protein")
                        nutrient_summary(preview: preview, code: "carbohydrate", label: "Carbs")
                        nutrient_summary(preview: preview, code: "fat", label: "Fat")
                        nutrient_summary(preview: preview, code: "fibre", label: "Fibre")
                    </div>
                </div>

                <details class="mt-2">
                    <summary class="cursor-pointer py-2 text-sm text-gray-600">"All nutrients"</summary>
                    <dl class="divide-y divide-gray-200">
                        for nutrient in &preview.nutrients {
                            <div class="flex justify-between gap-4 py-1.5 text-sm">
                                <dt class="text-gray-600">(&nutrient.name)</dt>
                                <dd
                                    data-nutrient-detail="true"
                                    data-basis-value=(nutrient.value)
                                    data-unit=(&nutrient.unit)
                                    class="shrink-0 tabular-nums text-gray-900"
                                >
                                    (format_nutrient(nutrient))
                                </dd>
                            </div>
                        }
                    </dl>
                </details>

                meal_selector(latest_meal: preview.latest_meal.as_ref())

                <button type="submit" class=(format!("mt-4 {}", button::OUTLINE_FULL))>
                    "Log food"
                </button>
            </form>
        </div>
    }
}

fn nutrient_value(preview: &FoodPreview, code: &str) -> Option<f64> {
    preview
        .nutrients
        .iter()
        .find(|nutrient| nutrient.code == code)
        .map(|nutrient| nutrient.value)
}

fn data_value(value: Option<f64>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

#[topcoat::view::component]
async fn nutrient_summary(preview: &FoodPreview, code: &str, label: &str) -> Result {
    let value = nutrient_value(preview, code);
    view! {
        <p>
            <span class="block text-xs text-gray-500">(label)</span>
            <strong data-nutrition=(code) class="text-sm font-medium tabular-nums">(format_optional(value)) " g"</strong>
        </p>
    }
}

pub(super) fn source_name(source: &str) -> &str {
    match source {
        "fineli" => "Fineli",
        "open_food_facts" => "Open Food Facts",
        "custom" => "Custom",
        _ => source,
    }
}

fn basis_name(unit: &str) -> &str {
    match unit {
        "g" => "per 100 g",
        "ml" => "per 100 ml",
        "count" => "per item",
        _ => unit,
    }
}

fn format_nutrient(nutrient: &NutrientValue) -> String {
    if nutrient.unit == "kJ" {
        return format!("{} kJ", format_number(nutrient.value));
    }
    if nutrient.value >= 1.0 {
        format!("{} g", format_number(nutrient.value))
    } else if nutrient.value >= 0.001 {
        format!("{} mg", format_number(nutrient.value * 1000.0))
    } else {
        format!("{} µg", format_number(nutrient.value * 1_000_000.0))
    }
}
