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
        path_param, query_params,
        response::{IntoResponse, Response},
        route,
    },
    view::{StaticClass, attributes, class, component, view},
};
use uuid::Uuid;

const DIARY_PENDING_TOTAL: StaticClass = class!(
    "transition-[opacity,filter] duration-100 ease-out \
     group-has-[.htmx-request]/diary:opacity-50 \
     group-has-[.htmx-request]/diary:blur-[1px] motion-reduce:transition-none",
);
const MEAL_PENDING_TOTAL: StaticClass = class!(
    "transition-[opacity,filter] duration-100 ease-out \
     group-has-[.htmx-request]/meal:opacity-50 \
     group-has-[.htmx-request]/meal:blur-[1px] motion-reduce:transition-none",
);

use crate::{
    auth,
    components::{
        button::{ButtonVariant, button},
        icon::{chevron_down_icon, trash_icon},
        input::input,
        select::select,
    },
    data::{
        Data, DiaryEntry, DiaryMeal, DiaryNutrient, FoodDiary, FoodNutrient, FoodPreview,
        FoodSearchResult, MealAction, MealChoice, NewFoodEntry,
    },
    day::Day,
    settings,
};

pub fn register(builder: RouterBuilder) -> RouterBuilder {
    builder
        .route(search_foods)
        .route(preview_food)
        .route(clear_food_preview)
        .route(add_food_entry)
        .route(update_food_entry)
        .route(delete_food_entry)
}

#[derive(Default)]
pub struct FoodSearch {
    pub query: String,
    pub results: Vec<FoodSearchResult>,
    pub preview: Option<FoodPreview>,
    pub meal_choice: Option<MealChoice>,
}

pub struct FoodLog<'a> {
    pub preview: Option<&'a FoodPreview>,
    pub meal_choice: Option<&'a MealChoice>,
    pub query: &'a str,
    pub day: Day,
    pub diary: &'a FoodDiary,
    pub timezone: Tz,
    pub added: bool,
}

#[topcoat::router::query_params(error = bad_request)]
struct HomeFoodQuery {
    q: Option<String>,
    food: Option<String>,
}

#[topcoat::router::query_params(error = bad_request)]
struct SearchQuery {
    q: Option<String>,
    date: Option<String>,
}

#[derive(Deserialize)]
struct FoodEntryForm {
    amount: f64,
    date: String,
    meal: String,
}

#[derive(Deserialize)]
struct EntryAmountForm {
    amount: f64,
    date: String,
}

#[derive(Deserialize)]
struct EntryDateForm {
    date: String,
}

path_param!(food_id: Uuid, error = bad_request);
path_param!(entry_id: Uuid, error = bad_request);

pub async fn load(cx: &Cx, user_id: Uuid, day: Day, timezone: &str) -> Result<FoodSearch> {
    let query = query_params::<HomeFoodQuery>(cx)?;
    let text = query.q.as_deref().unwrap_or_default().trim().to_owned();
    let results = data(cx).search_foods(user_id, &text).await?;
    let selected_food = match &query.food {
        Some(food_id) => {
            let food_id = food_id
                .parse()
                .map_err(|_| bad_request("food ID must be a UUID"))?;
            data(cx).food_preview(user_id, food_id).await?
        }
        None => None,
    };
    let meal_choice = if selected_food.is_some() {
        Some(data(cx).meal_choice(user_id, day.date, timezone).await?)
    } else {
        None
    };
    Ok(FoodSearch {
        query: text,
        results,
        preview: selected_food,
        meal_choice,
    })
}

#[route(GET "/foods/search")]
#[tracing::instrument(name = "food::search", level = "debug", skip_all)]
async fn search_foods(cx: &Cx) -> Result<Response> {
    let user = auth::require_user(cx).await?;
    let search = query_params::<SearchQuery>(cx)?;
    let query = search.q.as_deref().unwrap_or_default().trim().to_owned();
    let date = search
        .date
        .as_deref()
        .map(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d"))
        .transpose()
        .map_err(|_| bad_request("date must use YYYY-MM-DD"))?;
    let results = data(cx).search_foods(user.id, &query).await?;
    let fragment = view! {
        search_results(query: &query, results: &results, date: date.as_ref())
    }?;
    fragment.into_response(cx)
}

#[route(GET "/foods/{food_id}/preview")]
#[tracing::instrument(name = "food::preview", level = "debug", skip_all)]
async fn preview_food(cx: &Cx) -> Result<Response> {
    let user = auth::require_user(cx).await?;
    let timezone = user_timezone(&user)?;
    let day = Day::from_request(cx, timezone)?;
    let query = query_params::<SearchQuery>(cx)?
        .q
        .as_deref()
        .unwrap_or_default()
        .to_owned();
    let food_id = path_param::<FoodId>(cx)?;
    let food = data(cx).food_preview(user.id, *food_id).await?;
    let meal_choice = data(cx)
        .meal_choice(user.id, day.date, timezone.name())
        .await?;
    let fragment = view! {
        food_preview(
            food: food.as_ref(),
            meal_choice: Some(&meal_choice),
            query: &query,
            day: day
        )
    }?;
    fragment.into_response(cx)
}

#[route(GET "/foods/preview")]
#[tracing::instrument(name = "food::clear_preview", level = "debug", skip_all)]
async fn clear_food_preview(cx: &Cx) -> Result<Response> {
    let user = auth::require_user(cx).await?;
    let timezone = user_timezone(&user)?;
    let day = Day::from_request(cx, timezone)?;
    let query = query_params::<SearchQuery>(cx)?
        .q
        .as_deref()
        .unwrap_or_default()
        .to_owned();
    view! { food_preview(food: None, meal_choice: None, query: &query, day: day) }?
        .into_response(cx)
}

#[route(POST "/foods/{food_id}/entries")]
#[tracing::instrument(name = "food::add_entry", level = "debug", skip_all)]
async fn add_food_entry(cx: &Cx, Form(form): Form<FoodEntryForm>) -> Result<Response> {
    let user = auth::require_user(cx).await?;
    let timezone = user_timezone(&user)?;
    let timezone_name = timezone.name();
    let food_id = path_param::<FoodId>(cx)?;
    let date = NaiveDate::parse_from_str(&form.date, "%Y-%m-%d")
        .map_err(|_| bad_request("date must use YYYY-MM-DD"))?;
    validate_amount(form.amount)?;

    let meal = meal_action(&form.meal)?;
    let added = data(cx)
        .add_food_entry(NewFoodEntry {
            user_id: user.id,
            food_id: *food_id,
            amount: form.amount,
            date,
            timezone: timezone_name,
            meal,
        })
        .await?;
    if !added {
        return Err(bad_request("food or meal was not found").into());
    }

    let day = Day {
        date,
        today: Utc::now().with_timezone(&timezone).date_naive(),
    };
    if hx_request(cx) {
        let diary = data(cx).food_diary(user.id, date, timezone_name).await?;
        let food_log_state = FoodLog {
            preview: None,
            meal_choice: None,
            query: "",
            day,
            diary: &diary,
            timezone,
            added: true,
        };
        return view! { food_log(state: &food_log_state) }?.into_response(cx);
    }

    see_other(day.url()).into_response(cx)
}

#[route(POST "/food-entries/{entry_id}")]
#[tracing::instrument(name = "food::update_entry", level = "debug", skip_all)]
async fn update_food_entry(cx: &Cx, Form(form): Form<EntryAmountForm>) -> Result<Response> {
    let user = auth::require_user(cx).await?;
    let timezone = user_timezone(&user)?;
    let day = form_day(&form.date, timezone)?;
    validate_amount(form.amount)?;
    let entry_id = path_param::<EntryId>(cx)?;
    if !data(cx)
        .update_food_entry(user.id, *entry_id, form.amount)
        .await?
    {
        return Err(bad_request("food entry was not found").into());
    }
    if hx_request(cx) {
        let diary = data(cx)
            .food_diary(user.id, day.date, timezone.name())
            .await?;
        return view! { food_diary(diary: &diary, timezone: timezone, day: day) }?
            .into_response(cx);
    }
    see_other(day.url()).into_response(cx)
}

#[route(POST "/food-entries/{entry_id}/delete")]
#[tracing::instrument(name = "food::delete_entry", level = "debug", skip_all)]
async fn delete_food_entry(cx: &Cx, Form(form): Form<EntryDateForm>) -> Result<Response> {
    let user = auth::require_user(cx).await?;
    let timezone = user_timezone(&user)?;
    let day = form_day(&form.date, timezone)?;
    let entry_id = path_param::<EntryId>(cx)?;
    if !data(cx).delete_food_entry(user.id, *entry_id).await? {
        return Err(bad_request("food entry was not found").into());
    }
    if hx_request(cx) {
        let diary = data(cx)
            .food_diary(user.id, day.date, timezone.name())
            .await?;
        return view! { food_diary(diary: &diary, timezone: timezone, day: day) }?
            .into_response(cx);
    }
    see_other(day.url()).into_response(cx)
}

#[component]
pub async fn food_search(state: &FoodSearch, day: Day) -> Result {
    let date = day.date.to_string();
    view! {
        <section aria-labelledby="food-search-heading" class="mt-8">
            <h2 id="food-search-heading" class="sr-only">"Food search"</h2>
            <server-combobox class="relative block">
                <form
                    id="food-search-form"
                    method="get"
                    action="/"
                    hx-get="/foods/search"
                    hx-target="#food-results"
                    hx-swap="innerHTML"
                    hx-sync="this:replace"
                >
                    <input type="hidden" name="date" value=(&date)>
                    <label class="mb-2 block text-sm font-medium" for="food-query">
                        "Search foods"
                    </label>
                    <div class="flex gap-2">
                        <div class="relative min-w-0 flex-1">
                            <input
                                id="food-query"
                                data-combobox-input="true"
                                type="search"
                                name="q"
                                value=(&state.query)
                                autocomplete="off"
                                spellcheck="false"
                                placeholder="Food, brand, or barcode"
                                aria-controls="food-listbox"
                                hx-get="/foods/search"
                                hx-trigger="input changed delay:120ms, search"
                                hx-include="closest form"
                                hx-target="#food-results"
                                hx-swap="innerHTML"
                                hx-sync="closest form:replace"
                                class="w-full rounded-xl border border-gray-300 bg-white px-3 py-2.5 pr-11 text-base text-gray-950 outline-none placeholder:text-gray-400 hover:border-gray-400 focus:border-gray-700 focus:ring-2 focus:ring-gray-700/15 dark:border-gray-700 dark:bg-gray-900 dark:text-gray-100 dark:placeholder:text-gray-500 dark:hover:border-gray-600 dark:focus:border-gray-300 dark:focus:ring-gray-300/15"
                            >
                            <span
                                data-combobox-spinner="true"
                                aria-hidden="true"
                                class="pointer-events-none absolute inset-y-0 right-2 flex w-7 scale-90 items-center justify-center opacity-0 blur-[4px] transition-[opacity,scale,filter] duration-200 ease-out data-loading:scale-100 data-loading:opacity-100 data-loading:blur-none motion-reduce:transition-none"
                            >
                                <span
                                    class="size-4 animate-spin rounded-full border-2 border-gray-300 border-t-gray-900 motion-reduce:animate-none dark:border-gray-700 dark:border-t-gray-100"
                                ></span>
                            </span>
                        </div>
                        <button
                            type="submit"
                            class="min-w-20 rounded-xl border border-gray-300 px-3 py-2 text-sm font-medium outline-2 outline-transparent outline-offset-2 hover:bg-gray-100 focus-visible:outline-outline dark:border-gray-700 dark:hover:bg-gray-900"
                        >
                            "Search"
                        </button>
                    </div>
                </form>
                <div
                    id="food-results"
                    data-combobox-popup="true"
                    hidden=(state.query.is_empty())
                    class="z-20 mt-1 max-h-80 overflow-y-auto data-enhanced:rounded-xl data-enhanced:border data-enhanced:border-gray-200 data-enhanced:bg-white data-enhanced:p-1 data-enhanced:text-gray-950 data-enhanced:shadow-2xl data-enhanced:shadow-gray-950/15 dark:data-enhanced:border-gray-700 dark:data-enhanced:bg-gray-900 dark:data-enhanced:text-gray-100 dark:data-enhanced:shadow-black/40"
                >
                    search_results(
                        query: &state.query,
                        results: &state.results,
                        date: Some(&day.date)
                    )
                </div>
            </server-combobox>
        </section>
    }
}

#[component]
pub async fn food_log(state: &FoodLog<'_>) -> Result {
    view! {
        <div id="food-log" data-food-added=(if state.added { "true" } else { "false" })>
            food_preview(
                food: state.preview,
                meal_choice: state.meal_choice,
                query: state.query,
                day: state.day
            )
            food_diary(diary: state.diary, timezone: state.timezone, day: state.day)
        </div>
    }
}

#[component]
async fn search_results(
    query: &str,
    results: &[FoodSearchResult],
    date: Option<&NaiveDate>,
) -> Result {
    let query_length = query
        .chars()
        .filter(|character| character.is_alphanumeric())
        .count();
    let status = if query.is_empty() {
        String::new()
    } else if query_length < 2 {
        "Type at least two characters.".to_owned()
    } else if results.is_empty() {
        format!("No food matched “{query}”.")
    } else {
        format!(
            "{} {} available.",
            results.len(),
            if results.len() == 1 {
                "result"
            } else {
                "results"
            }
        )
    };

    view! {
        <ul id="food-listbox" data-combobox-list="true" class="m-0 list-none p-0">
            if query_length >= 2 {
                for result in results {
                    <li>
                        <button
                            type="submit"
                            form="food-search-form"
                            name="food"
                            value=(result.id.to_string())
                            formaction="/"
                            hx-get=(preview_url(result.id, query, date))
                            hx-target="#food-preview"
                            hx-swap="outerHTML"
                            hx-push-url=(selection_url(result.id, query, date))
                            data-combobox-option="true"
                            class="flex w-full items-center justify-between gap-4 rounded-lg px-3 py-2 text-left text-inherit outline-none hover:bg-gray-100 focus-visible:bg-gray-100 aria-selected:bg-gray-100 dark:hover:bg-gray-800 dark:focus-visible:bg-gray-800 dark:aria-selected:bg-gray-800"
                        >
                            <span class="min-w-0">
                                <span class="block truncate text-sm font-medium">
                                    (&result.name)
                                </span>
                                if let Some(brand) = &result.brand {
                                    <span
                                        class="mt-0.5 block truncate text-xs text-gray-500 dark:text-gray-400"
                                    >
                                        (brand)
                                    </span>
                                }
                            </span>
                            <span
                                class="shrink-0 text-right text-xs text-gray-500 dark:text-gray-400"
                            >
                                if let Some(energy) = result.energy_kcal {
                                    <span class="block tabular-nums">
                                        (format!("{energy:.0}"))
                                        " kcal"
                                    </span>
                                }
                                <span>(source_name(result.source.as_deref()))</span>
                            </span>
                        </button>
                    </li>
                }
            }
        </ul>
        <p
            data-combobox-status="true"
            role="status"
            aria-live="polite"
            aria-atomic="true"
            class=(if !query.is_empty()
                && (query_length < 2 || results.is_empty())
            {
                "px-3 py-2 text-sm text-gray-500 dark:text-gray-400"
            } else {
                "sr-only"
            })
        >
            (status)
        </p>
    }
}

#[component]
async fn food_preview(
    food: Option<&FoodPreview>,
    meal_choice: Option<&MealChoice>,
    query: &str,
    day: Day,
) -> Result {
    view! {
        if let (Some(food), Some(meal_choice)) = (food, meal_choice) {
            <section
                id="food-preview"
                aria-live="polite"
                aria-labelledby="food-preview-heading"
                class="mt-5 rounded-xl border border-gray-200 p-4 dark:border-gray-800"
            >
                <p
                    class="text-xs font-medium uppercase tracking-wider text-gray-500 dark:text-gray-400"
                >
                    (source_name(food.source.as_deref()))
                </p>
                <h3 id="food-preview-heading" class="mt-1 text-lg font-semibold">
                    (&food.name)
                </h3>
                if let Some(brand) = &food.brand {
                    <p class="mt-1 text-sm text-gray-500 dark:text-gray-400">(brand)</p>
                }
                <form
                    method="post"
                    action=(entry_url(food.id))
                    hx-post=(entry_url(food.id))
                    hx-target="#food-log"
                    hx-swap="outerHTML"
                    hx-push-url=(day.url())
                    data-food-entry-form="true"
                    class="mt-5"
                >
                    <input type="hidden" name="date" value=(day.date.to_string())>
                    <div data-food-calorie-field="true" class="w-32">
                        input(
                            label: Some("Amount"),
                            suffix: Some(&food.basis_unit),
                            attrs: attributes! {
                                type="number"
                                name="amount"
                                min="0.1"
                                max="100000"
                                step="any"
                                inputmode="decimal"
                                value="100"
                                required="required"
                                autofocus="autofocus"
                                class="[appearance:textfield] [&::-webkit-inner-spin-button]:m-0 [&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:m-0 [&::-webkit-outer-spin-button]:appearance-none"
                                data-food-amount="true"
                                data-food-calorie-input="true"
                                data-energy-per-hundred=(food
                                    .energy_kcal
                                    .map(|value| value.to_string())
                                    .unwrap_or_default())
                            }
                        )
                        <output
                            data-food-kcal-output="true"
                            class="mt-1 block text-right text-xs tabular-nums text-gray-500 dark:text-gray-400"
                        >
                            "-- kcal"
                        </output>
                    </div>
                    <label class="mt-4 block">
                        <span class="mb-2 block text-sm font-medium">"Meal"</span>
                        meal_select(choice: meal_choice)
                    </label>
                    <div class="mt-4 flex justify-end gap-2">
                        <a
                            href=(cancel_url(query, day.date))
                            hx-get=(clear_preview_url(query, day.date))
                            hx-target="#food-preview"
                            hx-swap="outerHTML"
                            hx-push-url=(cancel_url(query, day.date))
                            data-food-cancel="true"
                            class="inline-flex h-10 items-center rounded-lg px-4 text-sm font-medium text-gray-600 outline-2 outline-transparent outline-offset-2 hover:bg-gray-100 focus-visible:outline-outline dark:text-gray-400 dark:hover:bg-gray-800"
                        >
                            "Cancel"
                        </a>
                        button(variant: ButtonVariant::Primary, "Add food")
                    </div>
                </form>
                <div class="mt-5 border-t border-gray-200 pt-4 dark:border-gray-800">
                    <p
                        class="text-xs font-medium uppercase tracking-wider text-gray-500 dark:text-gray-400"
                    >
                        "Nutrition per 100 "
                        (&food.basis_unit)
                    </p>
                    <dl
                        class="mt-3 grid grid-cols-2 gap-x-4 gap-y-2 text-sm sm:grid-cols-5"
                    >
                        nutrient_summary(
                            label: "Energy",
                            value: food.energy_kcal,
                            unit: "kcal"
                        )
                        nutrient_summary(
                            label: "Protein",
                            value: food.protein,
                            unit: "g"
                        )
                        nutrient_summary(
                            label: "Carbs",
                            value: food.carbohydrate,
                            unit: "g"
                        )
                        nutrient_summary(label: "Fat", value: food.fat, unit: "g")
                        nutrient_summary(label: "Fibre", value: food.fibre, unit: "g")
                    </dl>
                    if food
                        .nutrients
                        .iter()
                        .any(|nutrient| !is_primary_nutrient(&nutrient.code)) {
                        <details class="mt-4 text-sm">
                            <summary
                                class="w-fit cursor-pointer text-gray-600 hover:text-gray-950 dark:text-gray-400 dark:hover:text-gray-100"
                            >
                                "All nutrients"
                            </summary>
                            <dl
                                class="mt-3 divide-y divide-gray-200 dark:divide-gray-800"
                            >
                                for nutrient in &food.nutrients {
                                    food_nutrient(nutrient: nutrient)
                                }
                            </dl>
                        </details>
                    }
                </div>
            </section>
        } else {
            <div id="food-preview" aria-live="polite"></div>
        }
    }
}

#[component]
async fn meal_select(choice: &MealChoice) -> Result {
    let options = view! {
        if let (Some(id), Some(name)) = (
            choice.latest_id,
            choice.latest_name.as_deref(),
        ) {
            <option value=(format!("continue:{id}")) selected=(choice.latest_is_active)>
                "Continue "
                (name)
            </option>
        }
        for name in ["Breakfast", "Lunch", "Snack", "Dinner"] {
            <option
                value=(name.to_lowercase())
                selected=(!choice.latest_is_active && choice.suggested_name == name)
            >
                (name)
            </option>
        }
    }?;

    view! {
        select(
            attrs: attributes! {
                id="meal-choice"
                name="meal"
                required="required"
                class="min-w-0"
            },
            (options)
        )
    }
}

#[component]
async fn food_nutrient(nutrient: &FoodNutrient) -> Result {
    view! {
        <div class="flex justify-between gap-4 py-1.5">
            <dt class="text-gray-600 dark:text-gray-400">(&nutrient.name)</dt>
            <dd class="shrink-0 font-medium tabular-nums">
                (nutrient_value(Some(nutrient.value), &nutrient.unit))
            </dd>
        </div>
    }
}

#[component]
pub async fn food_diary(diary: &FoodDiary, timezone: Tz, day: Day) -> Result {
    view! {
        <section
            id="food-diary"
            aria-labelledby="food-diary-heading"
            class="group/diary mt-8 border-t border-gray-200 pt-6 dark:border-gray-800"
        >
            <header class="flex items-baseline justify-between gap-4">
                <h2 id="food-diary-heading" class="text-lg font-semibold">"Food"</h2>
                <p
                    aria-label="Food energy total"
                    class=(class!(
                        "text-2xl font-semibold tabular-nums",
                        DIARY_PENDING_TOTAL,
                    ))
                >
                    (nutrient_value(diary.totals.energy_kcal, "kcal"))
                </p>
            </header>
            if diary.meals.is_empty() {
                <p class="mt-4 text-sm text-gray-500 dark:text-gray-400">
                    "No food logged."
                </p>
            } else {
                <dl
                    class=(class!(
                        "mt-4 grid grid-cols-4 gap-3 text-sm",
                        DIARY_PENDING_TOTAL,
                    ))
                >
                    diary_total(label: "Protein", value: diary.totals.protein)
                    diary_total(label: "Carbs", value: diary.totals.carbohydrate)
                    diary_total(label: "Fat", value: diary.totals.fat)
                    diary_total(label: "Fibre", value: diary.totals.fibre)
                </dl>
                if diary
                    .nutrients
                    .iter()
                    .any(|nutrient| !is_primary_nutrient(&nutrient.code)) {
                    <details class=(class!("mt-4 text-sm", DIARY_PENDING_TOTAL))>
                        <summary
                            class="w-fit cursor-pointer text-gray-600 hover:text-gray-950 dark:text-gray-400 dark:hover:text-gray-100"
                        >
                            "All daily nutrients"
                        </summary>
                        <dl class="mt-3 divide-y divide-gray-200 dark:divide-gray-800">
                            for nutrient in &diary.nutrients {
                                diary_nutrient(nutrient: nutrient)
                            }
                        </dl>
                    </details>
                }
                <div class="mt-6 space-y-7">
                    for meal in &diary.meals {
                        diary_meal(meal: meal, timezone: timezone, day: day)
                    }
                </div>
            }
        </section>
    }
}

#[component]
async fn diary_total(label: &str, value: Option<f64>) -> Result {
    view! {
        <div>
            <dt class="text-xs text-gray-500 dark:text-gray-400">(label)</dt>
            <dd class="mt-0.5 font-medium tabular-nums">
                (nutrient_value(value, "g"))
            </dd>
        </div>
    }
}

#[component]
async fn diary_nutrient(nutrient: &DiaryNutrient) -> Result {
    view! {
        <div class="flex justify-between gap-4 py-1.5">
            <dt class="text-gray-600 dark:text-gray-400">(&nutrient.name)</dt>
            <dd class="shrink-0 font-medium tabular-nums">
                (nutrient_value(nutrient.value, &nutrient.unit))
            </dd>
        </div>
    }
}

#[component]
async fn diary_meal(meal: &DiaryMeal, timezone: Tz, day: Day) -> Result {
    let local_time = meal.started_at.with_timezone(&timezone);
    view! {
        <article
            data-meal="true"
            class="group/meal has-[>ul>li:only-child.htmx-request]:hidden"
        >
            <header
                class="-mx-7 flex min-h-12 items-center justify-between gap-4 rounded-xl bg-gray-100 px-6 py-2 dark:bg-gray-900"
            >
                <h3 class="font-medium">(meal.name.as_deref().unwrap_or("Meal"))</h3>
                <p
                    class=(class!(
                        "shrink-0 text-right text-xs tabular-nums text-gray-500 dark:text-gray-400",
                        MEAL_PENDING_TOTAL,
                    ))
                >
                    <time datetime=(meal.started_at.to_rfc3339())>
                        (local_time.format("%H:%M").to_string())
                    </time>
                    <span aria-hidden="true">" · "</span>
                    <span>(nutrient_value(meal.energy_kcal, "kcal"))</span>
                </p>
            </header>
            <ul class="mt-2 divide-y divide-gray-200/70 dark:divide-gray-800/70">
                for entry in &meal.entries {
                    diary_entry(entry: entry, day: day)
                }
            </ul>
        </article>
    }
}

#[component]
async fn diary_entry(entry: &DiaryEntry, day: Day) -> Result {
    let row_id = format!("food-entry-{}", entry.id);
    let edit_form_id = format!("food-entry-edit-{}", entry.id);
    let energy_per_100 = entry
        .energy_kcal
        .map(|energy| energy * 100.0 / entry.amount);
    view! {
        <li id=(&row_id) class="py-0.5 [&.htmx-request]:hidden">
            <details
                data-food-entry-details="true"
                class="group -mx-3 rounded-lg open:bg-gray-100 dark:open:bg-gray-900"
            >
                <summary
                    class="grid cursor-pointer list-none grid-cols-[minmax(0,1fr)_auto_auto] items-start gap-3 rounded-lg px-3 py-3 outline-2 outline-transparent outline-offset-[-2px] hover:bg-gray-100 focus-visible:outline-outline group-open:hover:bg-transparent dark:hover:bg-gray-900 [&::-webkit-details-marker]:hidden"
                >
                    <span class="min-w-0">
                        <span class="block truncate text-sm font-medium">
                            (&entry.name)
                        </span>
                        if let Some(brand) = &entry.brand {
                            <span
                                class="mt-0.5 block truncate text-xs text-gray-500 dark:text-gray-400"
                            >
                                (brand)
                            </span>
                        }
                    </span>
                    <span
                        class="w-20 shrink-0 text-right text-xs text-gray-500 group-open:hidden dark:text-gray-400"
                    >
                        <span
                            class="block font-medium tabular-nums text-gray-700 dark:text-gray-300"
                        >
                            (format_amount(entry.amount))
                            " "
                            (&entry.unit)
                        </span>
                        <span class="tabular-nums">
                            (nutrient_value(entry.energy_kcal, "kcal"))
                        </span>
                    </span>
                    <span
                        data-food-calorie-field="true"
                        class="hidden w-20 shrink-0 group-open:block"
                    >
                        input(
                            suffix: Some(&entry.unit),
                            compact: true,
                            attrs: attributes! {
                                aria-label="Amount"
                                type="number"
                                name="amount"
                                form=(&edit_form_id)
                                min="0.1"
                                max="100000"
                                step="any"
                                inputmode="decimal"
                                value=(entry.amount.to_string())
                                required="required"
                                class="[appearance:textfield] [&::-webkit-inner-spin-button]:m-0 [&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:m-0 [&::-webkit-outer-spin-button]:appearance-none"
                                data-food-edit-amount="true"
                                data-food-calorie-input="true"
                                data-energy-per-hundred=(energy_per_100
                                    .map(|value| value.to_string())
                                    .unwrap_or_default())
                            }
                        )
                        <output
                            data-food-kcal-output="true"
                            class="mt-1 block text-right text-xs tabular-nums text-gray-500 dark:text-gray-400"
                        >
                            "-- kcal"
                        </output>
                    </span>
                    chevron_down_icon(
                        attrs: attributes! {
                            class="mt-3 shrink-0 text-gray-400 transition-transform duration-150 group-open:rotate-90 motion-reduce:transition-none"
                        }
                    )
                </summary>
                <form
                    id=(&edit_form_id)
                    method="post"
                    action=(entry_update_url(entry.id))
                    hx-post=(entry_update_url(entry.id))
                    hx-target="#food-diary"
                    hx-swap="outerHTML"
                    hx-push-url=(day.url())
                >
                    <input type="hidden" name="date" value=(day.date.to_string())>
                </form>
                <div class="flex items-center justify-between gap-2 px-3 pb-3">
                    <form
                        method="post"
                        action=(entry_delete_url(entry.id))
                        hx-post=(entry_delete_url(entry.id))
                        hx-target="#food-diary"
                        hx-swap="outerHTML"
                        hx-push-url=(day.url())
                        hx-indicator="closest li"
                    >
                        <input type="hidden" name="date" value=(day.date.to_string())>
                        <button
                            type="submit"
                            class="inline-flex h-10 items-center gap-2 rounded-lg px-3 text-sm font-medium text-red-700 outline-2 outline-transparent outline-offset-2 hover:bg-red-100 focus-visible:outline-outline dark:text-red-400 dark:hover:bg-red-950/50"
                        >
                            trash_icon()
                            "Delete"
                        </button>
                    </form>
                    <div class="flex items-center gap-2">
                        <a
                            href=(day.url())
                            data-food-edit-cancel="true"
                            class="inline-flex h-10 items-center rounded-lg px-3 text-sm font-medium text-gray-600 outline-2 outline-transparent outline-offset-2 hover:bg-gray-200 focus-visible:outline-outline dark:text-gray-400 dark:hover:bg-gray-700"
                        >
                            "Cancel"
                        </a>
                        button(
                            variant: ButtonVariant::Primary,
                            attrs: attributes! { type="submit" form=(&edit_form_id) class="px-4" },
                            "Save"
                        )
                    </div>
                </div>
            </details>
        </li>
    }
}

#[component]
async fn nutrient_summary(label: &str, value: Option<f64>, unit: &str) -> Result {
    view! {
        <div>
            <dt class="text-gray-500 dark:text-gray-400">(label)</dt>
            <dd class="font-medium tabular-nums">(nutrient_value(value, unit))</dd>
        </div>
    }
}

fn preview_url(food_id: Uuid, query: &str, date: Option<&NaiveDate>) -> String {
    let mut parameters = url::form_urlencoded::Serializer::new(String::new());
    if let Some(date) = date {
        parameters.append_pair("date", &date.to_string());
    }
    parameters.append_pair("q", query);
    format!("/foods/{food_id}/preview?{}", parameters.finish())
}

fn entry_url(food_id: Uuid) -> String {
    format!("/foods/{food_id}/entries")
}

fn entry_update_url(entry_id: Uuid) -> String {
    format!("/food-entries/{entry_id}")
}

fn entry_delete_url(entry_id: Uuid) -> String {
    format!("/food-entries/{entry_id}/delete")
}

fn clear_preview_url(query: &str, date: NaiveDate) -> String {
    let query = encoded_day_query(query, date);
    format!("/foods/preview?{query}")
}

fn cancel_url(query: &str, date: NaiveDate) -> String {
    format!("/?{}", encoded_day_query(query, date))
}

fn encoded_day_query(query: &str, date: NaiveDate) -> String {
    let mut parameters = url::form_urlencoded::Serializer::new(String::new());
    parameters
        .append_pair("date", &date.to_string())
        .append_pair("q", query)
        .finish()
}

fn meal_action(meal: &str) -> Result<MealAction<'_>> {
    if let Some(id) = meal.strip_prefix("continue:") {
        return Uuid::parse_str(id)
            .map(MealAction::Continue)
            .map_err(|_| bad_request("invalid meal").into());
    }
    meal_name(meal)
        .map(MealAction::Start)
        .ok_or_else(|| bad_request("invalid meal").into())
}

fn meal_name(value: &str) -> Option<&'static str> {
    match value.to_ascii_lowercase().as_str() {
        "breakfast" => Some("Breakfast"),
        "lunch" => Some("Lunch"),
        "snack" => Some("Snack"),
        "dinner" => Some("Dinner"),
        _ => None,
    }
}

fn is_primary_nutrient(code: &str) -> bool {
    matches!(
        code,
        "energy" | "protein" | "carbohydrate" | "fat" | "fibre"
    )
}

fn nutrient_value(value: Option<f64>, unit: &str) -> String {
    value.map_or_else(
        || "—".to_owned(),
        |value| {
            let places = if unit == "kcal" {
                0
            } else if value.abs() >= 10.0 {
                1
            } else if value.abs() >= 1.0 {
                2
            } else {
                3
            };
            format!("{} {unit}", format_decimal(value, places))
        },
    )
}

fn format_amount(value: f64) -> String {
    format_decimal(value, 2)
}

fn format_decimal(value: f64, places: usize) -> String {
    let value = format!("{value:.places$}");
    if places == 0 {
        value
    } else {
        value.trim_end_matches('0').trim_end_matches('.').to_owned()
    }
}

fn validate_amount(amount: f64) -> Result<()> {
    if amount.is_finite() && (0.1..=100_000.0).contains(&amount) {
        Ok(())
    } else {
        Err(bad_request("food amount must be between 0.1 and 100000").into())
    }
}

fn form_day(date: &str, timezone: Tz) -> Result<Day> {
    let date = NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| bad_request("date must use YYYY-MM-DD"))?;
    Ok(Day {
        date,
        today: Utc::now().with_timezone(&timezone).date_naive(),
    })
}

fn user_timezone(user: &auth::User) -> Result<Tz> {
    let Some(timezone) = user.timezone.as_deref() else {
        return Err(redirect(settings::setup_url("/")).into());
    };
    timezone
        .parse()
        .map_err(|_| bad_request("invalid user timezone").into())
}

fn selection_url(food_id: Uuid, query: &str, date: Option<&NaiveDate>) -> String {
    let mut parameters = url::form_urlencoded::Serializer::new(String::new());
    if let Some(date) = date {
        parameters.append_pair("date", &date.to_string());
    }
    let parameters = parameters
        .append_pair("q", query)
        .append_pair("food", &food_id.to_string())
        .finish();
    format!("/?{parameters}")
}

fn source_name(source: Option<&str>) -> &'static str {
    match source {
        Some("fineli") => "Fineli",
        Some("open_food_facts") => "Open Food Facts",
        _ => "Personal",
    }
}

fn data(cx: &Cx) -> &Data {
    app_context(cx)
}
