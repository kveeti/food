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

use crate::{auth, day::Day, db, ui::search_spinner};

pub const FOOD_JS: Asset = asset!("public/food.js");

pub fn register(builder: RouterBuilder) -> RouterBuilder {
    builder
        .route(search_foods)
        .route(preview_food)
        .route(preview_meal)
        .route(copy_meal)
        .route(log_food)
        .route(update_entry)
        .route(delete_entry)
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
struct LatestMeal {
    id: Uuid,
    name: Option<String>,
    local_time: String,
    active: bool,
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
        search(db(cx), user_id, food_query),
        search_meals(db(cx), user_id, food_query, timezone),
        load_day(db(cx), user_id, date, timezone),
        load_latest_meal(db(cx), user_id, date, timezone),
    )?;
    let preview = if let Some(food) = query.food.as_deref() {
        let food = Uuid::parse_str(food).map_err(|_| bad_request("invalid food"))?;
        let amount = parse_amount(query.amount.as_deref())?;
        Some(load_preview(cx, user_id, food, amount, date, latest_meal.clone()).await?)
    } else {
        None
    };

    let selected_meal_preview = if let Some(meal) = query.meal_preview.as_deref() {
        let meal = Uuid::parse_str(meal).map_err(|_| bad_request("invalid meal"))?;
        Some(load_meal_preview(db(cx), user_id, meal, date).await?)
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

async fn load_day(
    pool: &PgPool,
    user_id: Uuid,
    date: NaiveDate,
    timezone: &str,
) -> Result<(Vec<Meal>, DailyTotals)> {
    type EntryRow = (
        Uuid,
        Option<Uuid>,
        Option<String>,
        String,
        String,
        Option<String>,
        f64,
        String,
        DateTime<Utc>,
        String,
        Option<f64>,
    );

    let (entry_rows, total_rows): (Vec<EntryRow>, Vec<(String, f64, bool)>) =
        tokio::try_join!(
            sqlx::query_as(
                "SELECT entries.id, entries.meal_id, meals.name,
                        to_char(COALESCE(meals.started_at, entries.eaten_at) AT TIME ZONE $3, 'HH24:MI'),
                        entries.food_name, entries.food_brand, entries.amount, entries.unit,
                        entries.eaten_at,
                        to_char(entries.eaten_at AT TIME ZONE $3, 'HH24:MI'),
                        energy.consumed_value / 4.184
                 FROM food_entries entries
                 LEFT JOIN meals ON meals.id = entries.meal_id AND meals.user_id = entries.user_id
                 LEFT JOIN nutrients energy_name ON energy_name.code = 'energy'
                 LEFT JOIN food_entry_nutrients energy
                        ON energy.food_entry_id = entries.id
                       AND energy.nutrient_id = energy_name.id
                 WHERE entries.user_id = $1
                   AND entries.eaten_at >= ($2::date::timestamp AT TIME ZONE $3)
                   AND entries.eaten_at < (($2::date + 1)::timestamp AT TIME ZONE $3)
                 ORDER BY entries.meal_id IS NULL, meals.started_at, entries.eaten_at, entries.created_at",
            )
            .bind(user_id)
            .bind(date)
            .bind(timezone)
            .fetch_all(pool),
            sqlx::query_as(
                "WITH day_entries AS (
                     SELECT id
                     FROM food_entries
                     WHERE user_id = $1
                       AND eaten_at >= ($2::date::timestamp AT TIME ZONE $3)
                       AND eaten_at < (($2::date + 1)::timestamp AT TIME ZONE $3)
                 )
                 SELECT nutrients.code,
                        COALESCE(SUM(values.consumed_value), 0),
                        COUNT(values.food_entry_id) = (SELECT COUNT(*) FROM day_entries)
                 FROM nutrients
                 LEFT JOIN day_entries ON true
                 LEFT JOIN food_entry_nutrients values
                        ON values.food_entry_id = day_entries.id
                       AND values.nutrient_id = nutrients.id
                 WHERE nutrients.code = ANY($4)
                 GROUP BY nutrients.id, nutrients.code",
            )
            .bind(user_id)
            .bind(date)
            .bind(timezone)
            .bind(["energy", "protein", "carbohydrate", "fat", "fibre"])
            .fetch_all(pool),
        )?;

    let mut meals: Vec<Meal> = Vec::new();
    for row in entry_rows {
        if meals.last().is_none_or(|meal| meal.id != row.1) {
            meals.push(Meal {
                id: row.1,
                name: row.2.clone(),
                local_time: row.3.clone(),
                entries: Vec::new(),
                energy_kcal: 0.0,
                energy_complete: true,
            });
        }
        let meal = meals.last_mut().expect("meal was just added");
        if let Some(energy) = row.10 {
            meal.energy_kcal += energy;
        } else {
            meal.energy_complete = false;
        }
        meal.entries.push(FoodEntry {
            id: row.0,
            name: row.4,
            brand: row.5,
            amount: row.6,
            unit: row.7,
            eaten_at: row.8,
            local_time: row.9,
            energy_kcal: row.10,
        });
    }

    let total = |code: &str, energy: bool| {
        let row = total_rows.iter().find(|row| row.0 == code);
        NutrientTotal {
            value: row.map_or(0.0, |row| if energy { row.1 / 4.184 } else { row.1 }),
            complete: row.is_none_or(|row| row.2),
        }
    };
    let totals = DailyTotals {
        energy: total("energy", true),
        protein: total("protein", false),
        carbohydrate: total("carbohydrate", false),
        fat: total("fat", false),
        fibre: total("fibre", false),
    };

    Ok((meals, totals))
}

async fn load_latest_meal(
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

async fn search_meals(
    pool: &PgPool,
    user_id: Uuid,
    query: &str,
    timezone: &str,
) -> Result<Vec<MealSearchResult>> {
    if query
        .chars()
        .filter(|character| !character.is_whitespace())
        .count()
        < 3
    {
        return Ok(Vec::new());
    }

    let rows: Vec<(Uuid, Option<String>, String, String, Vec<String>)> = sqlx::query_as(
        "WITH input AS (
             SELECT normalized_query
             FROM (
                 SELECT trim(regexp_replace(lower($2), '[^[:alnum:]]+', ' ', 'g')) AS normalized_query
             ) AS normalized_input
             WHERE char_length(replace(normalized_query, ' ', '')) >= 3
         ), meal_data AS (
             SELECT meals.id, meals.name, meals.started_at,
                    MAX(entries.eaten_at) AS latest_entry,
                    lower(concat_ws(
                        ' ', meals.name,
                        string_agg(concat_ws(' ', entries.food_name, entries.food_brand), ' ')
                    )) AS search_text,
                    (array_agg(entries.food_name ORDER BY entries.eaten_at, entries.created_at))[1:3] AS foods
             FROM meals
             JOIN food_entries entries ON entries.meal_id = meals.id
             WHERE meals.user_id = $1 AND entries.user_id = $1
             GROUP BY meals.id
         )
         SELECT meal_data.id, meal_data.name,
                to_char(meal_data.started_at AT TIME ZONE $3, 'Mon FMDD, YYYY'),
                to_char(meal_data.started_at AT TIME ZONE $3, 'HH24:MI'),
                meal_data.foods
         FROM meal_data
         CROSS JOIN input
         WHERE NOT EXISTS (
             SELECT 1
             FROM unnest(regexp_split_to_array(input.normalized_query, ' ')) AS term
             WHERE meal_data.search_text NOT LIKE ('%' || term || '%')
         )
         ORDER BY
             CASE
                 WHEN lower(COALESCE(meal_data.name, '')) = input.normalized_query THEN 0
                 WHEN lower(COALESCE(meal_data.name, '')) LIKE (input.normalized_query || '%') THEN 1
                 ELSE 2
             END,
             meal_data.latest_entry DESC
         LIMIT 5",
    )
    .bind(user_id)
    .bind(query)
    .bind(timezone)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(id, name, local_date, local_time, foods)| MealSearchResult {
                id,
                name,
                local_date,
                local_time,
                foods,
            },
        )
        .collect())
}

async fn load_meal_preview(
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

async fn load_preview(
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
    let timezone = user.timezone.ok_or_redirect("/settings?required=1")?;
    let query = query_params::<FoodSearchQuery>(cx)?;
    let food_query = query.food_query.as_deref().unwrap_or_default().trim();
    let date = query
        .date
        .as_deref()
        .map(parse_date)
        .transpose()?
        .ok_or_else(|| bad_request("date is required"))?;
    let (results, meal_results) = tokio::try_join!(
        search(db(cx), user.id, food_query),
        search_meals(db(cx), user.id, food_query, &timezone),
    )?;
    let fragment = view! {
        search_results(results: &results, meal_results: &meal_results, date: date)
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
    let latest_meal = load_latest_meal(db(cx), user.id, date, &timezone).await?;
    let preview = load_preview(cx, user.id, *food, amount, date, latest_meal).await?;
    let fragment = view! {
        food_preview(preview: &preview)
    }?;
    fragment.into_response(cx)
}

#[path_param(error = bad_request)]
struct MealId(Uuid);

#[route(GET "/meals/{meal_id}/preview")]
async fn preview_meal(cx: &Cx) -> Result<Response> {
    let user = auth::require_user(cx).await?;
    user.timezone.ok_or_redirect("/settings?required=1")?;
    let meal = path_param::<MealId>(cx)?;
    let query = query_params::<PreviewQuery>(cx)?;
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

fn parse_date(date: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| bad_request("date must use YYYY-MM-DD").into())
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

#[path_param(error = bad_request)]
struct EntryId(Uuid);

#[derive(Debug, serde::Deserialize)]
struct UpdateEntryForm {
    amount: String,
    date: String,
}

#[route(POST "/food-entries/{entry_id}")]
async fn update_entry(cx: &Cx, Form(input): Form<UpdateEntryForm>) -> Result<Response> {
    let user = auth::require_user(cx).await?;
    user.timezone.ok_or_redirect("/settings?required=1")?;
    let entry_id = path_param::<EntryId>(cx)?;
    let amount = parse_required_amount(&input.amount)?;
    let date = parse_date(&input.date)?;
    let mut transaction = db(cx).begin().await?;
    let unit: Option<String> = sqlx::query_scalar(
        "UPDATE food_entries
         SET amount = $1
         WHERE id = $2 AND user_id = $3
         RETURNING unit",
    )
    .bind(amount)
    .bind(*entry_id)
    .bind(user.id)
    .fetch_optional(&mut *transaction)
    .await?;
    let Some(unit) = unit else {
        return Err(not_found().into());
    };
    let factor = if unit == "count" {
        amount
    } else {
        amount / 100.0
    };
    sqlx::query(
        "UPDATE food_entry_nutrients
         SET consumed_value = basis_value * $1
         WHERE food_entry_id = $2",
    )
    .bind(factor)
    .bind(*entry_id)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;

    redirect_to_day(cx, date)
}

#[derive(Debug, serde::Deserialize)]
struct DeleteEntryForm {
    date: String,
}

#[route(POST "/food-entries/{entry_id}/delete")]
async fn delete_entry(cx: &Cx, Form(input): Form<DeleteEntryForm>) -> Result<Response> {
    let user = auth::require_user(cx).await?;
    user.timezone.ok_or_redirect("/settings?required=1")?;
    let entry_id = path_param::<EntryId>(cx)?;
    let date = parse_date(&input.date)?;
    let mut transaction = db(cx).begin().await?;
    let meal_id: Option<Uuid> = sqlx::query_scalar(
        "DELETE FROM food_entries
         WHERE id = $1 AND user_id = $2
         RETURNING meal_id",
    )
    .bind(*entry_id)
    .bind(user.id)
    .fetch_optional(&mut *transaction)
    .await?
    .flatten();
    if let Some(meal_id) = meal_id {
        sqlx::query(
            "DELETE FROM meals
             WHERE id = $1 AND user_id = $2
               AND NOT EXISTS (
                   SELECT 1 FROM food_entries WHERE meal_id = meals.id
               )",
        )
        .bind(meal_id)
        .bind(user.id)
        .execute(&mut *transaction)
        .await?;
    }
    transaction.commit().await?;

    redirect_to_day(cx, date)
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
pub async fn food_search(day: &Day, food: &FoodHome) -> Result {
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
                    class="relative min-w-23 rounded-lg border border-gray-200 px-3 py-2 text-sm font-medium text-gray-800 hover:bg-gray-150 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-gray-700"
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
                search_results(results: &food.results, meal_results: &food.meal_results, date: day.date)
            </div>

            if let Some(preview) = &food.preview {
                food_preview(preview: preview)
            } else if let Some(preview) = &food.meal_preview {
                meal_preview(preview: preview)
            } else {
                <div id="food-preview" class="mt-5"></div>
            }

            daily_totals(totals: &food.totals)
            food_diary(meals: &food.meals, date: day.date)
        </section>
    }
}

#[topcoat::view::component]
async fn daily_totals(totals: &DailyTotals) -> Result {
    let incomplete = !totals.energy.complete
        || !totals.protein.complete
        || !totals.carbohydrate.complete
        || !totals.fat.complete
        || !totals.fibre.complete;

    view! {
        <div aria-label="Daily nutrition totals" class="mt-6 grid grid-cols-3 gap-x-3 gap-y-3 border-y border-gray-200 py-3">
            daily_total(label: "Energy", total: &totals.energy, unit: "kcal")
            daily_total(label: "Protein", total: &totals.protein, unit: "g")
            daily_total(label: "Carbs", total: &totals.carbohydrate, unit: "g")
            daily_total(label: "Fat", total: &totals.fat, unit: "g")
            daily_total(label: "Fibre", total: &totals.fibre, unit: "g")
            if incomplete {
                <p class="self-end text-xs text-gray-500">"* Incomplete"</p>
            }
        </div>
    }
}

#[topcoat::view::component]
async fn daily_total(label: &str, total: &NutrientTotal, unit: &str) -> Result {
    view! {
        <p>
            <span class="block text-xs text-gray-500">(label)</span>
            <strong class="text-sm font-medium tabular-nums">(total_text(total, unit))</strong>
        </p>
    }
}

#[topcoat::view::component]
async fn food_diary(meals: &[Meal], date: NaiveDate) -> Result {
    view! {
        <div id="food-diary" class="mt-5">
            if meals.is_empty() {
                <p class="py-3 text-center text-sm text-gray-500">"No food logged"</p>
            } else {
                for meal in meals {
                    <section class="border-b border-gray-200 py-3 first:border-t" aria-label=(meal_name(meal.name.as_deref()))>
                        <header class="mb-1 flex items-start justify-between gap-4">
                            <div>
                                <h3 class="text-sm font-medium text-gray-900">
                                    (meal_name(meal.name.as_deref())) " · " (&meal.local_time)
                                </h3>
                                <p class="text-xs tabular-nums text-gray-500">
                                    (format_number(meal.energy_kcal)) " kcal"
                                    if !meal.energy_complete { "*" }
                                </p>
                            </div>
                            if let Some(meal_id) = meal.id {
                                <a
                                    href=(meal_url(meal_id, date))
                                    hx-get=(meal_preview_url(meal_id, date))
                                    hx-push-url=(meal_url(meal_id, date))
                                    hx-target="#food-preview"
                                    hx-swap="outerHTML show:top showTarget:#food-preview"
                                    aria-label=(format!("Copy {} meal", meal_name(meal.name.as_deref())))
                                    class="grid size-8 shrink-0 place-items-center rounded-lg border border-gray-200 text-gray-600 hover:bg-gray-150 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-gray-700"
                                >
                                    <svg aria-hidden="true" width="15" height="15" viewBox="0 0 15 15" fill="none" xmlns="http://www.w3.org/2000/svg">
                                        <path d="M1 9.50006C1 10.3285 1.67157 11.0001 2.5 11.0001H4L4 10.0001H2.5C2.22386 10.0001 2 9.7762 2 9.50006L2 2.50006C2 2.22392 2.22386 2.00006 2.5 2.00006L9.5 2.00006C9.77614 2.00006 10 2.22392 10 2.50006V4.00002H5.5C4.67158 4.00002 4 4.67159 4 5.50002V12.5C4 13.3284 4.67158 14 5.5 14H12.5C13.3284 14 14 13.3284 14 12.5V5.50002C14 4.67159 13.3284 4.00002 12.5 4.00002H11V2.50006C11 1.67163 10.3284 1.00006 9.5 1.00006H2.5C1.67157 1.00006 1 1.67163 1 2.50006V9.50006ZM5 5.50002C5 5.22388 5.22386 5.00002 5.5 5.00002H12.5C12.7761 5.00002 13 5.22388 13 5.50002V12.5C13 12.7762 12.7761 13 12.5 13H5.5C5.22386 13 5 12.7762 5 12.5V5.50002Z" fill="currentColor" fill-rule="evenodd" clip-rule="evenodd"></path>
                                    </svg>
                                </a>
                            }
                        </header>
                        <ul>
                            for entry in &meal.entries {
                                food_entry(entry: entry, date: date)
                            }
                        </ul>
                    </section>
                }
            }
        </div>
    }
}

#[topcoat::view::component]
async fn food_entry(entry: &FoodEntry, date: NaiveDate) -> Result {
    let amount = format!(
        "{} {}",
        format_amount(entry.amount),
        entry_unit(&entry.unit)
    );
    let energy = entry
        .energy_kcal
        .map(|value| format!("{} kcal", format_number(value)))
        .unwrap_or_else(|| "— kcal".to_owned());

    view! {
        <li class="border-t border-gray-200 first:border-t-0">
            <details>
                <summary class="flex min-h-11 cursor-pointer list-none items-center justify-between gap-3 py-2">
                    <span class="min-w-0">
                        <span class="block truncate text-sm text-gray-900">(&entry.name)</span>
                        if let Some(brand) = &entry.brand {
                            <span class="block truncate text-xs text-gray-500">(brand)</span>
                        }
                    </span>
                    <span class="flex shrink-0 items-center gap-3 text-right text-xs tabular-nums text-gray-500">
                        <span>
                            <span class="block">(amount)</span>
                            <span class="block">(energy)</span>
                        </span>
                        <time datetime=(entry.eaten_at.to_rfc3339())>(&entry.local_time)</time>
                    </span>
                </summary>
                <div class="flex items-end gap-2 pb-3">
                    <form method="post" action=(format!("/food-entries/{}", entry.id)) class="flex min-w-0 flex-1 items-end gap-2">
                        <input type="hidden" name="date" value=(date.to_string())>
                        <label class="min-w-0 flex-1 text-xs text-gray-600">
                            <span class="mb-1 block">"Amount in " (unit_name(&entry.unit))</span>
                            <input
                                name="amount"
                                type="number"
                                inputmode="decimal"
                                min="0"
                                max="100000"
                                step="any"
                                required="true"
                                value=(entry.amount)
                                class="w-full rounded-lg border border-gray-300 bg-form px-3 py-2 text-base text-gray-1000 outline-none focus:border-gray-500 focus:ring-1 focus:ring-gray-400"
                            >
                        </label>
                        <button type="submit" class="rounded-lg border border-gray-200 px-3 py-2 text-sm font-medium text-gray-800 hover:bg-gray-150 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-gray-700">"Save"</button>
                    </form>
                    <form method="post" action=(format!("/food-entries/{}/delete", entry.id))>
                        <input type="hidden" name="date" value=(date.to_string())>
                        <button
                            type="submit"
                            aria-label=(format!("Delete {}", entry.name))
                            class="grid size-9 place-items-center rounded-lg border border-gray-200 text-gray-500 hover:bg-danger-surface hover:text-danger-text focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-danger-text"
                        >
                            <svg aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" class="size-4">
                                <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12"></path>
                            </svg>
                        </button>
                    </form>
                </div>
            </details>
        </li>
    }
}

#[topcoat::view::component]
async fn search_results(
    results: &[SearchResult],
    meal_results: &[MealSearchResult],
    date: NaiveDate,
) -> Result {
    view! {
        if !results.is_empty() {
            if !meal_results.is_empty() {
                <p class="mb-1 text-xs font-medium text-gray-500">"Foods"</p>
            }
            <ul class="overflow-hidden rounded-lg border border-gray-200">
                for result in results {
                    <li class="border-t border-gray-200 first:border-t-0">
                        <a
                            href=(food_url(result.id, date))
                            hx-get=(preview_url(result.id, date))
                            hx-target="#food-preview"
                            hx-swap="outerHTML"
                            data-result-type="food"
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

        if !meal_results.is_empty() {
            <p class="mb-1 mt-3 text-xs font-medium text-gray-500">"Meals"</p>
            <ul class="overflow-hidden rounded-lg border border-gray-200">
                for meal in meal_results {
                    <li class="border-t border-gray-200 first:border-t-0">
                        <a
                            href=(meal_url(meal.id, date))
                            hx-get=(meal_preview_url(meal.id, date))
                            hx-push-url=(meal_url(meal.id, date))
                            hx-target="#food-preview"
                            hx-swap="outerHTML"
                            data-result-type="meal"
                            class="flex items-center justify-between gap-4 px-3 py-2.5 hover:bg-gray-100 focus-visible:bg-gray-100 focus-visible:outline-none"
                        >
                            <span class="min-w-0">
                                <span class="block truncate text-sm font-medium text-gray-900">(meal_name(meal.name.as_deref()))</span>
                                <span class="block truncate text-xs text-gray-500">(meal.foods.join(", "))</span>
                            </span>
                            <span class="shrink-0 text-right text-xs text-gray-500">
                                <span class="block">(&meal.local_date)</span>
                                <span class="tabular-nums">(&meal.local_time)</span>
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

fn meal_url(meal_id: Uuid, date: NaiveDate) -> String {
    format!("/?date={date}&meal_preview={meal_id}#food-preview")
}

fn meal_preview_url(meal_id: Uuid, date: NaiveDate) -> String {
    format!("/meals/{meal_id}/preview?date={date}")
}

fn cancel_meal_copy_url(date: NaiveDate) -> String {
    format!("/?date={date}#food-section")
}

#[topcoat::view::component]
async fn meal_preview(preview: &MealPreview) -> Result {
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
                        class="w-full rounded-lg border border-gray-300 bg-form px-3 py-2 text-base text-gray-1000 outline-none focus:border-gray-500 focus:ring-1 focus:ring-gray-400"
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
                        class="inline-flex shrink-0 items-center justify-center rounded-lg px-3 py-2 text-sm font-medium text-gray-800 hover:bg-gray-150 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-gray-700"
                    >
                        "Cancel"
                    </a>
                    <button type="submit" class="flex-1 rounded-lg border border-gray-200 px-3 py-2 text-sm font-medium text-gray-800 hover:bg-gray-150 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-gray-700">
                        "Add meal"
                    </button>
                </div>
            </form>
        </div>
    }
}

#[topcoat::view::component]
async fn food_preview(preview: &FoodPreview) -> Result {
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
                        class="w-full rounded-lg border border-gray-300 bg-form px-3 py-2 text-base text-gray-1000 outline-none focus:border-gray-500 focus:ring-1 focus:ring-gray-400"
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

                <div class="mt-4 grid gap-3 sm:grid-cols-2">
                    <label class="text-sm text-gray-700">
                        <span class="mb-1 block">"Meal"</span>
                        <select name="meal" class="w-full rounded-lg border border-gray-300 bg-form px-3 py-2 text-base text-gray-1000 outline-none focus:border-gray-500 focus:ring-1 focus:ring-gray-400">
                            <option value="new" selected=(preview.latest_meal.as_ref().is_none_or(|meal| !meal.active))>"Start a new meal"</option>
                            if let Some(meal) = &preview.latest_meal {
                                <option value=(meal.id.to_string()) selected=(meal.active)>
                                    "Continue " (meal_name(meal.name.as_deref())) " · " (&meal.local_time)
                                </option>
                            }
                        </select>
                    </label>
                    <label class="text-sm text-gray-700">
                        <span class="mb-1 block">"New meal name (optional)"</span>
                        <input
                            name="meal_name"
                            list="meal-name-suggestions"
                            maxlength="100"
                            autocomplete="off"
                            placeholder="Meal"
                            class="w-full rounded-lg border border-gray-300 bg-form px-3 py-2 text-base text-gray-1000 outline-none placeholder:text-gray-500 focus:border-gray-500 focus:ring-1 focus:ring-gray-400"
                        >
                    </label>
                </div>
                <datalist id="meal-name-suggestions">
                    <option value="Breakfast"></option>
                    <option value="Lunch"></option>
                    <option value="Dinner"></option>
                    <option value="Snack"></option>
                </datalist>

                <button type="submit" class="mt-4 w-full rounded-lg border border-gray-200 px-3 py-2 text-sm font-medium text-gray-800 hover:bg-gray-150 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-gray-700">
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

fn source_name(source: &str) -> &str {
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

fn meal_name(name: Option<&str>) -> &str {
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
