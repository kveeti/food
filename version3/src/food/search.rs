use super::*;

pub(super) fn register(builder: RouterBuilder) -> RouterBuilder {
    builder.route(search_foods)
}

pub(super) async fn search(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    query: &str,
) -> Result<Vec<SearchResult>> {
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

pub(super) async fn search_meals(
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

#[topcoat::view::component]
pub(super) async fn search_results(
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
                            hx-get=(preview::preview_url(result.id, date))
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
                                <span>(preview::source_name(&result.source))</span>
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
                            href=(meals::meal_url(meal.id, date))
                            hx-get=(meals::meal_preview_url(meal.id, date))
                            hx-push-url=(meals::meal_url(meal.id, date))
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
