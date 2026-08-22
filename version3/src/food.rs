use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{PgPool, types::Uuid};
use topcoat::{
    Result,
    asset::{Asset, asset},
    context::Cx,
    router::{
        HeaderValue, IntoResponse, Response, RouterBuilder, StatusCode,
        content::Form,
        error::{RouterErrorExt, bad_request, not_found},
        header, path_param, query_params, route,
    },
    view::view,
};

use crate::{
    auth,
    components::{button, search_spinner},
    day::Day,
    db,
};

mod diary;
mod meals;
mod preview;
mod search;

pub const FOOD_JS: Asset = asset!("public/food.js");

pub fn register(builder: RouterBuilder) -> RouterBuilder {
    let builder = search::register(builder);
    let builder = preview::register(builder);
    let builder = meals::register(builder);
    diary::register(builder)
}

#[derive(Debug)]
struct SearchResult {
    id: Uuid,
    name: String,
    brand: Option<String>,
    source: String,
    energy_kcal: Option<f64>,
}

#[derive(Debug)]
struct MealSearchResult {
    id: Uuid,
    name: Option<String>,
    local_date: String,
    local_time: String,
    foods: Vec<String>,
}

#[derive(Debug)]
struct NutrientValue {
    code: String,
    name: String,
    value: f64,
    unit: String,
}

#[derive(Clone, Debug)]
pub(crate) struct LatestMeal {
    pub(crate) id: Uuid,
    pub(crate) name: Option<String>,
    pub(crate) local_time: String,
    pub(crate) active: bool,
}

#[derive(Debug)]
struct FoodPreview {
    id: Uuid,
    name: String,
    brand: Option<String>,
    source: String,
    basis_unit: String,
    amount: f64,
    date: NaiveDate,
    nutrients: Vec<NutrientValue>,
    latest_meal: Option<LatestMeal>,
}

#[derive(Debug)]
struct MealPreviewEntry {
    id: Uuid,
    name: String,
    brand: Option<String>,
    amount: f64,
    unit: String,
    available: bool,
}

#[derive(Debug)]
struct MealPreview {
    id: Uuid,
    date: NaiveDate,
    name: Option<String>,
    entries: Vec<MealPreviewEntry>,
}

#[derive(Debug)]
struct FoodEntry {
    id: Uuid,
    name: String,
    brand: Option<String>,
    amount: f64,
    unit: String,
    eaten_at: DateTime<Utc>,
    local_time: String,
    energy_kcal: Option<f64>,
}

#[derive(Debug)]
struct Meal {
    id: Option<Uuid>,
    name: Option<String>,
    local_time: String,
    entries: Vec<FoodEntry>,
    energy_kcal: f64,
    energy_complete: bool,
}

#[derive(Debug)]
struct NutrientTotal {
    value: f64,
    complete: bool,
}

#[derive(Debug)]
struct DailyTotals {
    energy: NutrientTotal,
    protein: NutrientTotal,
    carbohydrate: NutrientTotal,
    fat: NutrientTotal,
    fibre: NutrientTotal,
}

#[derive(Debug)]
pub struct FoodHome {
    query: String,
    results: Vec<SearchResult>,
    meal_results: Vec<MealSearchResult>,
    preview: Option<FoodPreview>,
    meal_preview: Option<MealPreview>,
    meals: Vec<Meal>,
    totals: DailyTotals,
    latest_meal: Option<LatestMeal>,
}

#[topcoat::router::query_params(error = bad_request)]
struct HomeFoodQuery {
    food_query: Option<String>,
    food: Option<String>,
    amount: Option<String>,
    meal_preview: Option<String>,
}

pub async fn home_state(
    cx: &Cx,
    user_id: Uuid,
    date: NaiveDate,
    timezone: &str,
) -> Result<FoodHome> {
    let query = query_params::<HomeFoodQuery>(cx)?;
    let food_query = query.food_query.as_deref().unwrap_or_default().trim();
    let (results, meal_results, (meals, totals), latest_meal) = tokio::try_join!(
        search::search(db(cx), user_id, food_query),
        search::search_meals(db(cx), user_id, food_query, timezone),
        diary::load_day(db(cx), user_id, date, timezone),
        meals::load_latest_meal(db(cx), user_id, date, timezone),
    )?;
    let preview = if let Some(food) = query.food.as_deref() {
        let food = Uuid::parse_str(food).map_err(|_| bad_request("invalid food"))?;
        let amount = parse_amount(query.amount.as_deref())?;
        Some(preview::load_preview(cx, user_id, food, amount, date, latest_meal.clone()).await?)
    } else {
        None
    };

    let selected_meal_preview = if let Some(meal) = query.meal_preview.as_deref() {
        let meal = Uuid::parse_str(meal).map_err(|_| bad_request("invalid meal"))?;
        Some(meals::load_meal_preview(db(cx), user_id, meal, date).await?)
    } else {
        None
    };

    Ok(FoodHome {
        query: food_query.to_owned(),
        results,
        meal_results,
        preview,
        meal_preview: selected_meal_preview,
        meals,
        totals,
        latest_meal,
    })
}

fn parse_amount(amount: Option<&str>) -> Result<Option<f64>> {
    let Some(amount) = amount else {
        return Ok(None);
    };
    let amount = amount
        .parse::<f64>()
        .map_err(|_| bad_request("amount must be a number"))?;
    if !amount.is_finite() || !(0.001..=100_000.0).contains(&amount) {
        return Err(bad_request("amount must be between 0.001 and 100000").into());
    }
    Ok(Some(amount))
}

fn parse_date(date: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| bad_request("date must use YYYY-MM-DD").into())
}

fn parse_required_amount(amount: &str) -> Result<f64> {
    parse_amount(Some(amount))?.ok_or_else(|| bad_request("amount is required").into())
}

fn parse_meal_name(name: Option<&str>) -> Result<Option<String>> {
    let name = name.map(str::trim).filter(|name| !name.is_empty());
    if name.is_some_and(|name| name.chars().count() > 100) {
        return Err(bad_request("meal name must be at most 100 characters").into());
    }
    Ok(name.map(str::to_owned))
}

fn redirect_to_day(cx: &Cx, date: NaiveDate) -> Result<Response> {
    let location = format!("/?date={date}#food-section");
    (
        StatusCode::SEE_OTHER,
        [(header::LOCATION, HeaderValue::from_str(&location)?)],
        (),
    )
        .into_response(cx)
}

#[topcoat::view::component]
pub async fn food_section(day: &Day, food: &FoodHome) -> Result {
    let energy_total = total_text(&food.totals.energy, "kcal");

    view! {
        <section id="food-section" aria-labelledby="food-heading" class="mt-10">
            <div class="mb-3 flex items-baseline justify-between gap-4">
                <h2 id="food-heading" class="text-base font-medium text-gray-1000">"Food"</h2>
                <span aria-label="Food energy total" class="text-sm font-medium tabular-nums text-gray-900">(energy_total)</span>
            </div>
            <form method="get" action="/" class="flex gap-2">
                <input type="hidden" name="date" value=(day.date.to_string())>
                <label for="food-query" class="sr-only">"Search foods"</label>
                <input
                    id="food-query"
                    name="food_query"
                    type="search"
                    value=(&food.query)
                    autocomplete="off"
                    minlength="3"
                    placeholder="Search foods"
                    hx-get="/foods/search"
                    hx-trigger="input changed delay:100ms, search"
                    hx-include="closest form"
                    hx-target="#food-results"
                    hx-swap="innerHTML"
                    hx-sync="closest form:replace"
                    hx-indicator="#food-search-button"
                    class="min-w-0 flex-1 rounded-lg border border-gray-300 bg-form px-3 py-2.5 text-base text-gray-1000 outline-none placeholder:text-gray-500 hover:border-gray-400 focus:border-gray-500 focus:ring-1 focus:ring-gray-400"
                >
                <button
                    id="food-search-button"
                    type="submit"
                    hx-get="/foods/search"
                    hx-include="closest form"
                    hx-target="#food-results"
                    hx-swap="innerHTML"
                    hx-sync="closest form:replace"
                    hx-indicator="#food-search-button"
                    class=(format!("relative min-w-23 {}", button::OUTLINE))
                >
                    <span class="search-label">"Search"</span>
                    search_spinner()
                </button>
            </form>

            <p class="mt-2 text-xs text-gray-500">
                if let Some(meal) = &food.latest_meal && meal.active {
                    "Continuing " (meal_name(meal.name.as_deref())) " · " (&meal.local_time)
                } else {
                    "The next food starts a new meal"
                }
            </p>

            <div id="food-results" class="mt-2">
                search::search_results(results: &food.results, meal_results: &food.meal_results, date: day.date)
            </div>

            if let Some(preview) = &food.preview {
                preview::food_preview(preview: preview)
            } else if let Some(preview) = &food.meal_preview {
                meals::meal_preview(preview: preview)
            } else {
                <div id="food-preview" class="mt-5"></div>
            }

            diary::daily_totals(totals: &food.totals)
            diary::food_diary(meals: &food.meals, date: day.date)
        </section>
    }
}

fn unit_name(unit: &str) -> &str {
    match unit {
        "g" => "grams",
        "ml" => "millilitres",
        "count" => "items",
        _ => unit,
    }
}

fn entry_unit(unit: &str) -> &str {
    match unit {
        "count" => "items",
        _ => unit,
    }
}

pub(crate) fn meal_name(name: Option<&str>) -> &str {
    name.unwrap_or("Meal")
}

fn total_text(total: &NutrientTotal, unit: &str) -> String {
    let incomplete = if total.complete { "" } else { "*" };
    format!("{}{} {}", format_number(total.value), incomplete, unit)
}

fn format_amount(value: f64) -> String {
    if value.fract().abs() < f64::EPSILON {
        format!("{value:.0}")
    } else {
        format_number(value)
    }
}

fn format_optional(value: Option<f64>) -> String {
    value.map_or_else(|| "—".to_owned(), format_number)
}

fn format_number(value: f64) -> String {
    if value == 0.0 {
        "0".to_owned()
    } else if value >= 100.0 {
        format!("{value:.0}")
    } else if value >= 10.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.2}")
    }
}
