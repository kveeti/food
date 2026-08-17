use chrono::NaiveDate;
use sqlx::types::Uuid;
use topcoat::{
    Result,
    context::Cx,
    router::{
        IntoResponse, Response, RouterBuilder,
        error::{RouterErrorExt, bad_request, not_found},
        path_param, query_params, route,
    },
    view::view,
};

use crate::{auth, day::Day, db, ui::search_spinner};

pub fn register(builder: RouterBuilder) -> RouterBuilder {
    builder.route(search_foods).route(preview_food)
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
struct NutrientValue {
    code: String,
    name: String,
    value: f64,
    unit: String,
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
}

#[derive(Debug)]
pub struct FoodHome {
    query: String,
    results: Vec<SearchResult>,
    preview: Option<FoodPreview>,
}

#[topcoat::router::query_params(error = bad_request)]
struct HomeFoodQuery {
    food_query: Option<String>,
    food: Option<String>,
    amount: Option<String>,
}

pub async fn home_state(cx: &Cx, user_id: Uuid, date: NaiveDate) -> Result<FoodHome> {
    let query = query_params::<HomeFoodQuery>(cx)?;
    let food_query = query.food_query.as_deref().unwrap_or_default().trim();
    let results = search(db(cx), user_id, food_query).await?;
    let preview = if let Some(food) = query.food.as_deref() {
        let food = Uuid::parse_str(food).map_err(|_| bad_request("invalid food"))?;
        let amount = parse_amount(query.amount.as_deref())?;
        Some(load_preview(cx, user_id, food, amount, date).await?)
    } else {
        None
    };

    Ok(FoodHome {
        query: food_query.to_owned(),
        results,
        preview,
    })
}

async fn search(pool: &sqlx::PgPool, user_id: Uuid, query: &str) -> Result<Vec<SearchResult>> {
    if query.is_empty() {
        return Ok(Vec::new());
    }
    let rows = sqlx::query_as(
        "WITH normalized_input AS (
             SELECT trim(regexp_replace(lower($2), '[^[:alnum:]]+', ' ', 'g')) AS normalized_query
         ), input AS (
             SELECT normalized_query,
                    regexp_split_to_array(normalized_query, ' ') AS terms,
                    plainto_tsquery('simple', $2) AS simple_exact_query,
                    plainto_tsquery('finnish', $2) AS fi_query,
                    plainto_tsquery('swedish', $2) AS sv_query,
                    plainto_tsquery('english', $2) AS en_query
             FROM normalized_input
             WHERE char_length(replace(normalized_query, ' ', '')) >= 3
         ), search AS (
             SELECT input.*,
                    to_tsquery('simple', (
                        SELECT string_agg(
                            CASE
                                WHEN query_term.position = cardinality(input.terms)
                                THEN query_term.term || ':*'
                                ELSE query_term.term
                            END,
                            ' & ' ORDER BY query_term.position
                        )
                        FROM unnest(input.terms) WITH ORDINALITY AS query_term(term, position)
                    )) AS simple_prefix_query
             FROM input
         ), candidates AS (
             SELECT foods.id, foods.display_name, foods.brand, foods.source,
                    foods.source_data, foods.search_vector,
                    foods.search_fi_vector, foods.search_sv_vector, foods.search_en_vector,
                    energy.value / 4.184 AS energy_kcal,
                    search.*,
                    trim(regexp_replace(lower(foods.display_name), '[^[:alnum:]]+', ' ', 'g')) AS normalized_name,
                    trim(regexp_replace(lower(split_part(foods.display_name, ',', 1)), '[^[:alnum:]/]+', ' ', 'g')) AS primary_name,
                    trim(regexp_replace(lower(COALESCE(foods.brand, '')), '[^[:alnum:]]+', ' ', 'g')) AS normalized_brand
             FROM foods
             CROSS JOIN search
             LEFT JOIN nutrients energy_name ON energy_name.code = 'energy'
             LEFT JOIN food_nutrients energy
                    ON energy.food_id = foods.id AND energy.nutrient_id = energy_name.id
             WHERE NOT foods.is_archived
               AND (foods.source <> 'custom' OR foods.owner_user_id = $1)
               AND (
                   foods.search_vector @@ search.simple_prefix_query
                   OR foods.search_fi_vector @@ search.fi_query
                   OR foods.search_sv_vector @@ search.sv_query
                   OR foods.search_en_vector @@ search.en_query
               )
         ), title_scores AS (
             SELECT candidates.*,
                    regexp_split_to_array(normalized_name, ' ') AS name_words,
                    EXISTS (
                        SELECT 1
                        FROM regexp_split_to_table(primary_name, '/') AS alternative(name)
                        WHERE trim(alternative.name) = normalized_query
                    ) AS primary_exact,
                    EXISTS (
                        SELECT 1
                        FROM regexp_split_to_table(primary_name, '/') AS alternative(name)
                        WHERE trim(alternative.name) LIKE (normalized_query || '%')
                    ) AS primary_prefix
             FROM candidates
         ), field_scores AS (
             SELECT title_scores.*,
                    exact_title.distance AS exact_title_distance,
                    prefix_title.distance AS prefix_title_distance,
                    COALESCE(alias_scores.alias_exact, false) AS alias_exact,
                    alias_scores.alias_distance,
                    COALESCE(alias_scores.alias_prefix, false) AS alias_prefix
             FROM title_scores
             LEFT JOIN LATERAL (
                 SELECT max(term_match.position) AS distance
                 FROM unnest(terms) AS query_term(term)
                 CROSS JOIN LATERAL (
                     SELECT min(name_word.position)::integer AS position
                     FROM unnest(name_words) WITH ORDINALITY AS name_word(word, position)
                     WHERE name_word.word = query_term.term
                 ) AS term_match
                 HAVING count(term_match.position) = cardinality(terms)
             ) AS exact_title ON true
             LEFT JOIN LATERAL (
                 SELECT max(term_match.position) AS distance
                 FROM unnest(terms) WITH ORDINALITY AS query_term(term, query_position)
                 CROSS JOIN LATERAL (
                     SELECT min(name_word.position)::integer AS position
                     FROM unnest(name_words) WITH ORDINALITY AS name_word(word, position)
                     WHERE name_word.word = query_term.term
                        OR (
                            query_term.query_position = cardinality(terms)
                            AND name_word.word LIKE (query_term.term || '%')
                        )
                 ) AS term_match
                 HAVING count(term_match.position) = cardinality(terms)
             ) AS prefix_title ON true
             LEFT JOIN LATERAL (
                 SELECT bool_or(alias_name = normalized_query) AS alias_exact,
                        min((
                            SELECT max(term_match.position)
                            FROM unnest(terms) AS query_term(term)
                            CROSS JOIN LATERAL (
                                SELECT min(alias_word.position)::integer AS position
                                FROM unnest(alias_words) WITH ORDINALITY AS alias_word(word, position)
                                WHERE alias_word.word = query_term.term
                            ) AS term_match
                            HAVING count(term_match.position) = cardinality(terms)
                        )) AS alias_distance,
                        bool_or(alias_name LIKE (normalized_query || '%')) AS alias_prefix
                 FROM (
                     SELECT trim(regexp_replace(lower(food_aliases.name), '[^[:alnum:]]+', ' ', 'g')) AS alias_name,
                            regexp_split_to_array(
                                trim(regexp_replace(lower(food_aliases.name), '[^[:alnum:]]+', ' ', 'g')),
                                ' '
                            ) AS alias_words
                     FROM food_aliases
                     WHERE food_aliases.food_id = title_scores.id
                 ) AS aliases
             ) AS alias_scores ON true
         ), ranked AS (
             SELECT field_scores.*,
                    CASE
                        WHEN normalized_name = normalized_query THEN 0
                        WHEN primary_exact
                          OR alias_exact
                          OR normalized_brand = normalized_query THEN 1
                        WHEN exact_title_distance IS NOT NULL OR alias_distance IS NOT NULL THEN 2
                        WHEN primary_prefix THEN 3
                        WHEN prefix_title_distance IS NOT NULL OR alias_prefix THEN 4
                        WHEN normalized_brand LIKE (normalized_query || ' %') THEN 5
                        ELSE 6
                    END AS match_class,
                    LEAST(
                        COALESCE(exact_title_distance, 32767),
                        COALESCE(alias_distance, 32767),
                        COALESCE(prefix_title_distance, 32767)
                    ) AS match_distance,
                    ts_rank_cd(search_vector, simple_exact_query)
                    + ts_rank_cd(search_vector, simple_prefix_query)
                    + ts_rank_cd(search_fi_vector, fi_query)
                    + ts_rank_cd(search_sv_vector, sv_query)
                    + ts_rank_cd(search_en_vector, en_query) AS relevance
             FROM field_scores
         )
         SELECT id, display_name, brand, source, energy_kcal
         FROM ranked
         ORDER BY
             CASE WHEN match_class <= 1 THEN match_class ELSE 2 END,
             CASE
                 WHEN match_class > 1 AND source_data->>'food_type' = 'DISH' THEN 1
                 ELSE 0
             END,
             match_class,
             match_distance,
             CASE WHEN lower(display_name) LIKE '%, keskiarvo,%' THEN 0 ELSE 1 END,
             CASE COALESCE(source_data->>'process', '')
                 WHEN 'RAW' THEN 0
                 WHEN 'IND' THEN 1
                 WHEN '' THEN 1
                 ELSE 2
             END,
             CASE WHEN match_class <= 4
                  THEN CASE source WHEN 'custom' THEN 0 WHEN 'fineli' THEN 1 ELSE 2 END
             END,
             CASE WHEN match_class <= 4 THEN char_length(display_name) END,
             relevance DESC,
             CASE source WHEN 'custom' THEN 0 WHEN 'fineli' THEN 1 ELSE 2 END,
             display_name
         LIMIT 10",
    )
    .bind(user_id)
    .bind(query)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(id, name, brand, source, energy_kcal)| SearchResult {
            id,
            name,
            brand,
            source,
            energy_kcal,
        })
        .collect())
}

async fn load_preview(
    cx: &Cx,
    user_id: Uuid,
    food_id: Uuid,
    amount: Option<f64>,
    date: NaiveDate,
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
    let factor = if first.3 == "count" {
        amount
    } else {
        amount / 100.0
    };
    let nutrients = rows
        .iter()
        .map(|row| NutrientValue {
            code: row.4.clone(),
            name: row.5.clone(),
            unit: row.6.clone(),
            value: row.7 * factor,
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

#[topcoat::router::query_params(error = bad_request)]
struct FoodSearchQuery {
    food_query: Option<String>,
    date: Option<String>,
}

#[route(GET "/foods/search")]
async fn search_foods(cx: &Cx) -> Result<Response> {
    let user = auth::require_user(cx).await?;
    user.timezone.ok_or_redirect("/settings?required=1")?;
    let query = query_params::<FoodSearchQuery>(cx)?;
    let food_query = query.food_query.as_deref().unwrap_or_default().trim();
    let date = query
        .date
        .as_deref()
        .map(parse_date)
        .transpose()?
        .ok_or_else(|| bad_request("date is required"))?;
    let results = search(db(cx), user.id, food_query).await?;
    let fragment = view! {
        search_results(results: &results, date: date)
    }?;
    fragment.into_response(cx)
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
    user.timezone.ok_or_redirect("/settings?required=1")?;
    let food = path_param::<FoodId>(cx)?;
    let query = query_params::<PreviewQuery>(cx)?;
    let amount = parse_amount(query.amount.as_deref())?;
    let date = query
        .date
        .as_deref()
        .map(parse_date)
        .transpose()?
        .ok_or_else(|| bad_request("date is required"))?;
    let preview = load_preview(cx, user.id, *food, amount, date).await?;
    let fragment = view! {
        food_preview(preview: &preview)
    }?;
    fragment.into_response(cx)
}

fn parse_date(date: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| bad_request("date must use YYYY-MM-DD").into())
}

#[topcoat::view::component]
pub async fn food_search(day: &Day, food: &FoodHome) -> Result {
    view! {
        <section aria-labelledby="food-heading" class="mt-10">
            <div class="mb-3 flex items-baseline justify-between">
                <h2 id="food-heading" class="text-base font-medium text-gray-1000">"Food"</h2>
                <span class="text-xs text-gray-500">"Preview only"</span>
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
                    class="relative min-w-23 rounded-lg border border-gray-300 px-4 py-2.5 text-sm font-medium text-gray-800 hover:bg-gray-100 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-gray-500"
                >
                    <span class="search-label">"Search"</span>
                    search_spinner()
                </button>
            </form>

            <div id="food-results" class="mt-2">
                search_results(results: &food.results, date: day.date)
            </div>

            if let Some(preview) = &food.preview {
                food_preview(preview: preview)
            } else {
                <div id="food-preview" class="mt-5"></div>
            }
        </section>
    }
}

#[topcoat::view::component]
async fn search_results(results: &[SearchResult], date: NaiveDate) -> Result {
    view! {
        if !results.is_empty() {
            <ul class="overflow-hidden rounded-lg border border-gray-200">
                for result in results {
                    <li class="border-t border-gray-200 first:border-t-0">
                        <a
                            href=(food_url(result.id, date))
                            hx-get=(preview_url(result.id, date))
                            hx-target="#food-preview"
                            hx-swap="outerHTML"
                            class="flex items-center justify-between gap-4 px-3 py-2.5 hover:bg-gray-100 focus-visible:bg-gray-100 focus-visible:outline-none"
                        >
                            <span class="min-w-0">
                                <span class="block truncate text-sm font-medium text-gray-900">(&result.name)</span>
                                if let Some(brand) = &result.brand {
                                    <span class="block truncate text-xs text-gray-500">(brand)</span>
                                }
                            </span>
                            <span class="shrink-0 text-right text-xs text-gray-500">
                                if let Some(energy) = result.energy_kcal {
                                    <span class="block tabular-nums">(format_number(energy)) " kcal"</span>
                                }
                                <span>(source_name(&result.source))</span>
                            </span>
                        </a>
                    </li>
                }
            </ul>
        }
    }
}

fn food_url(food_id: Uuid, date: NaiveDate) -> String {
    format!("/?date={date}&food={food_id}")
}

fn preview_url(food_id: Uuid, date: NaiveDate) -> String {
    format!("/foods/{food_id}/preview?date={date}")
}

#[topcoat::view::component]
async fn food_preview(preview: &FoodPreview) -> Result {
    let energy = preview
        .nutrients
        .iter()
        .find(|nutrient| nutrient.code == "energy")
        .map(|nutrient| nutrient.value / 4.184);

    view! {
        <div id="food-preview" class="mt-5 rounded-xl border border-gray-200 p-4">
            <div class="mb-4">
                <p class="text-xs text-gray-500">(source_name(&preview.source))</p>
                <h3 class="text-base font-semibold text-gray-1000">(&preview.name)</h3>
                if let Some(brand) = &preview.brand {
                    <p class="text-sm text-gray-600">(brand)</p>
                }
            </div>

            <form method="get" action="/" class="mb-4 flex items-end gap-2">
                <input type="hidden" name="food" value=(preview.id.to_string())>
                <input type="hidden" name="date" value=(preview.date.to_string())>
                <label for="food-amount" class="min-w-0 flex-1 text-sm text-gray-700">
                    <span class="mb-1 block">"Amount in " (unit_name(&preview.basis_unit))</span>
                    <input
                        id="food-amount"
                        name="amount"
                        type="number"
                        inputmode="decimal"
                        min="0.001"
                        max="100000"
                        step="any"
                        required="true"
                        value=(preview.amount)
                        hx-get=(preview_url(preview.id, preview.date))
                        hx-trigger="input changed delay:100ms"
                        hx-target="#food-preview"
                        hx-swap="outerHTML"
                        hx-sync="this:replace"
                        class="w-full rounded-lg border border-gray-300 bg-form px-3 py-2 text-base text-gray-1000 outline-none focus:border-gray-500 focus:ring-1 focus:ring-gray-400"
                    >
                </label>
                <button type="submit" class="rounded-lg border border-gray-300 px-3 py-2 text-sm font-medium text-gray-800 hover:bg-gray-100">
                    "Preview"
                </button>
            </form>

            <div class="grid grid-cols-3 gap-2 border-y border-gray-200 py-3">
                <p>
                    <span class="block text-xs text-gray-500">"Energy"</span>
                    <strong class="text-sm font-medium tabular-nums">(format_optional(energy)) " kcal"</strong>
                </p>
                nutrient_summary(preview: preview, code: "protein", label: "Protein")
                nutrient_summary(preview: preview, code: "carbohydrate", label: "Carbs")
                nutrient_summary(preview: preview, code: "fat", label: "Fat")
                nutrient_summary(preview: preview, code: "fibre", label: "Fibre")
            </div>

            <details class="mt-2">
                <summary class="cursor-pointer py-2 text-sm text-gray-600">"All nutrients"</summary>
                <dl class="divide-y divide-gray-200">
                    for nutrient in &preview.nutrients {
                        <div class="flex justify-between gap-4 py-1.5 text-sm">
                            <dt class="text-gray-600">(&nutrient.name)</dt>
                            <dd class="shrink-0 tabular-nums text-gray-900">(format_nutrient(nutrient))</dd>
                        </div>
                    }
                </dl>
            </details>

            <p class="mt-3 text-xs text-gray-500">"Preview only — nothing will be saved."</p>
        </div>
    }
}

#[topcoat::view::component]
async fn nutrient_summary(preview: &FoodPreview, code: &str, label: &str) -> Result {
    let value = preview
        .nutrients
        .iter()
        .find(|nutrient| nutrient.code == code)
        .map(|nutrient| nutrient.value);
    view! {
        <p>
            <span class="block text-xs text-gray-500">(label)</span>
            <strong class="text-sm font-medium tabular-nums">(format_optional(value)) " g"</strong>
        </p>
    }
}

fn source_name(source: &str) -> &str {
    match source {
        "fineli" => "Fineli",
        "open_food_facts" => "Open Food Facts",
        "custom" => "Custom",
        _ => source,
    }
}

fn unit_name(unit: &str) -> &str {
    match unit {
        "g" => "grams",
        "ml" => "millilitres",
        "count" => "count",
        _ => unit,
    }
}

fn format_optional(value: Option<f64>) -> String {
    value.map_or_else(|| "—".to_owned(), format_number)
}

fn format_number(value: f64) -> String {
    if value >= 100.0 {
        format!("{value:.0}")
    } else if value >= 10.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.2}")
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
