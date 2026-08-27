use std::collections::HashMap;

use chrono::NaiveDate;
use sqlx::types::Uuid;
use topcoat::{
    Result,
    context::Cx,
    router::{
        IntoResponse, Response, RouterBuilder,
        content::Form,
        error::{bad_request, see_other},
        route,
    },
    view::view,
};

use crate::{
    auth,
    components::{button, input, select_control},
    db,
};

const DEFAULT_NUTRIENTS: [&str; 4] = ["protein", "carbohydrate", "fat", "fibre"];

#[derive(Clone, Debug)]
pub struct NutrientGoal {
    pub code: String,
    pub name: String,
    pub target: f64,
}

#[derive(Clone, Debug, Default)]
pub struct DailyGoals {
    pub food_kcal: Option<f64>,
    pub water_ml: Option<i64>,
    pub nutrients: Vec<NutrientGoal>,
}

#[derive(Debug)]
pub struct GoalSettings {
    effective_from: NaiveDate,
    daily_burn_kcal: Option<i32>,
    adjustment_kind: &'static str,
    adjustment_amount: Option<i32>,
    water_goal_ml: Option<i32>,
    nutrient_goals: Vec<NutrientGoal>,
    available_nutrients: Vec<String>,
}

pub fn register(builder: RouterBuilder) -> RouterBuilder {
    builder.route(save_goals)
}

pub async fn load_for_date(cx: &Cx, user_id: Uuid, date: NaiveDate) -> Result<DailyGoals> {
    let profile: Option<(Uuid, Option<i32>, i32, Option<i32>)> = sqlx::query_as(
        "SELECT id, daily_burn_kcal, food_adjustment_kcal, water_goal_ml
         FROM goal_profiles
         WHERE user_id = $1 AND effective_from <= $2
         ORDER BY effective_from DESC
         LIMIT 1",
    )
    .bind(user_id)
    .bind(date)
    .fetch_optional(db(cx))
    .await?;

    let Some((profile_id, burn, adjustment, water)) = profile else {
        return Ok(DailyGoals::default());
    };
    let nutrients = load_nutrient_goals(cx, profile_id).await?;

    Ok(DailyGoals {
        food_kcal: burn.map(|burn| f64::from(burn + adjustment)),
        water_ml: water.map(i64::from),
        nutrients,
    })
}

pub async fn water_goal_for_date(cx: &Cx, user_id: Uuid, date: NaiveDate) -> Result<Option<i64>> {
    let goal: Option<Option<i32>> = sqlx::query_scalar(
        "SELECT water_goal_ml
         FROM goal_profiles
         WHERE user_id = $1 AND effective_from <= $2
         ORDER BY effective_from DESC
         LIMIT 1",
    )
    .bind(user_id)
    .bind(date)
    .fetch_optional(db(cx))
    .await?;
    Ok(goal.flatten().map(i64::from))
}

pub async fn load_settings(cx: &Cx, user_id: Uuid, today: NaiveDate) -> Result<GoalSettings> {
    let profile: Option<(Uuid, Option<i32>, i32, Option<i32>)> = sqlx::query_as(
        "SELECT id, daily_burn_kcal, food_adjustment_kcal, water_goal_ml
         FROM goal_profiles
         WHERE user_id = $1 AND effective_from <= $2
         ORDER BY effective_from DESC
         LIMIT 1",
    )
    .bind(user_id)
    .bind(today)
    .fetch_optional(db(cx))
    .await?;

    let (burn, adjustment, water, nutrient_goals) =
        if let Some((id, burn, adjustment, water)) = profile {
            (burn, adjustment, water, load_nutrient_goals(cx, id).await?)
        } else {
            (None, 0, None, Vec::new())
        };
    let available_nutrients = sqlx::query_scalar(
        "SELECT display_name
         FROM nutrients
         WHERE code <> 'energy' AND NOT is_archived AND NOT (code = ANY($1))
         ORDER BY display_order, display_name",
    )
    .bind(DEFAULT_NUTRIENTS)
    .fetch_all(db(cx))
    .await?;
    let (adjustment_kind, adjustment_amount) = match adjustment.cmp(&0) {
        std::cmp::Ordering::Less => ("deficit", Some(adjustment.unsigned_abs() as i32)),
        std::cmp::Ordering::Greater => ("surplus", Some(adjustment)),
        std::cmp::Ordering::Equal => ("maintain", None),
    };

    Ok(GoalSettings {
        effective_from: today,
        daily_burn_kcal: burn,
        adjustment_kind,
        adjustment_amount,
        water_goal_ml: water,
        nutrient_goals,
        available_nutrients,
    })
}

async fn load_nutrient_goals(cx: &Cx, profile_id: Uuid) -> Result<Vec<NutrientGoal>> {
    Ok(sqlx::query_as::<_, (String, String, f64)>(
        "SELECT nutrients.code, nutrients.display_name, goals.target_value
         FROM nutrient_goals goals
         JOIN nutrients ON nutrients.id = goals.nutrient_id
         WHERE goals.goal_profile_id = $1
         ORDER BY nutrients.display_order, nutrients.display_name",
    )
    .bind(profile_id)
    .fetch_all(db(cx))
    .await?
    .into_iter()
    .map(|(code, name, target)| NutrientGoal { code, name, target })
    .collect())
}

#[route(POST "/settings/goals")]
async fn save_goals(cx: &Cx, Form(input): Form<HashMap<String, String>>) -> Result<Response> {
    let user = auth::require_user(cx).await?;
    user.timezone
        .ok_or_else(|| bad_request("timezone is required"))?;
    let effective_from = input
        .get("effective_from")
        .ok_or_else(|| bad_request("starting date is required"))
        .and_then(|date| {
            NaiveDate::parse_from_str(date, "%Y-%m-%d")
                .map_err(|_| bad_request("starting date must use YYYY-MM-DD"))
        })?;
    let burn = optional_positive_integer(input.get("daily_burn_kcal"), "daily burn")?;
    let water = optional_positive_integer(input.get("water_goal_ml"), "water goal")?;
    let adjustment_amount =
        optional_positive_integer(input.get("adjustment_amount"), "adjustment amount")?;
    let adjustment = match input.get("adjustment_kind").map(String::as_str) {
        Some("maintain") | None => 0,
        Some("deficit") => {
            -adjustment_amount.ok_or_else(|| bad_request("deficit amount is required"))?
        }
        Some("surplus") => {
            adjustment_amount.ok_or_else(|| bad_request("surplus amount is required"))?
        }
        _ => return Err(bad_request("invalid adjustment").into()),
    };
    if burn.is_none() && adjustment != 0 {
        return Err(bad_request("daily burn is required for an adjustment").into());
    }
    if burn.is_some_and(|burn| burn + adjustment <= 0) {
        return Err(bad_request("food goal must be greater than zero").into());
    }

    let mut targets = HashMap::<String, f64>::new();
    for (key, value) in &input {
        let Some(code) = key.strip_prefix("nutrient_") else {
            continue;
        };
        if value.trim().is_empty() {
            continue;
        }
        let target = value
            .trim()
            .parse::<f64>()
            .map_err(|_| bad_request("nutrient goal must be a number"))?;
        if !target.is_finite() || target < 0.0 {
            return Err(bad_request("nutrient goal cannot be negative").into());
        }
        if target > 0.0 {
            targets.insert(code.to_owned(), target);
        }
    }

    let added_name = input
        .get("add_nutrient_name")
        .map_or("", |value| value.trim());
    let added_target = input
        .get("add_nutrient_target")
        .map_or("", |value| value.trim());
    if !added_name.is_empty() || !added_target.is_empty() {
        if added_name.is_empty() || added_target.is_empty() {
            return Err(bad_request("nutrient name and goal are both required").into());
        }
        let matches: Vec<String> = sqlx::query_scalar(
            "SELECT code
             FROM nutrients
             WHERE code <> 'energy' AND NOT is_archived
               AND lower(display_name) = lower($1)",
        )
        .bind(added_name)
        .fetch_all(db(cx))
        .await?;
        if matches.len() != 1 {
            return Err(bad_request("choose a nutrient from the list").into());
        }
        targets.insert(
            matches[0].clone(),
            positive_number(added_target, "nutrient goal")?,
        );
    }

    let codes: Vec<String> = targets.keys().cloned().collect();
    let nutrient_rows: Vec<(Uuid, String)> = if codes.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as("SELECT id, code FROM nutrients WHERE code = ANY($1)")
            .bind(&codes)
            .fetch_all(db(cx))
            .await?
    };
    if nutrient_rows.len() != codes.len() {
        return Err(bad_request("unknown nutrient goal").into());
    }
    let nutrient_ids: Vec<Uuid> = nutrient_rows.iter().map(|row| row.0).collect();
    let target_values: Vec<f64> = nutrient_rows.iter().map(|row| targets[&row.1]).collect();

    let mut transaction = db(cx).begin().await?;
    let profile_id: Uuid = sqlx::query_scalar(
        "INSERT INTO goal_profiles (
             user_id, effective_from, daily_burn_kcal, food_adjustment_kcal, water_goal_ml
         )
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (user_id, effective_from)
         DO UPDATE SET daily_burn_kcal = EXCLUDED.daily_burn_kcal,
                       food_adjustment_kcal = EXCLUDED.food_adjustment_kcal,
                       water_goal_ml = EXCLUDED.water_goal_ml,
                       updated_at = now()
         RETURNING id",
    )
    .bind(user.id)
    .bind(effective_from)
    .bind(burn)
    .bind(adjustment)
    .bind(water)
    .fetch_one(&mut *transaction)
    .await?;
    sqlx::query("DELETE FROM nutrient_goals WHERE goal_profile_id = $1")
        .bind(profile_id)
        .execute(&mut *transaction)
        .await?;
    if !nutrient_ids.is_empty() {
        sqlx::query(
            "INSERT INTO nutrient_goals (goal_profile_id, nutrient_id, target_value)
             SELECT $1, nutrient_id, target_value
             FROM unnest($2::uuid[], $3::double precision[]) AS goals(nutrient_id, target_value)",
        )
        .bind(profile_id)
        .bind(nutrient_ids)
        .bind(target_values)
        .execute(&mut *transaction)
        .await?;
    }
    transaction.commit().await?;

    Ok(see_other("/settings#goals").into_response(cx)?)
}

fn optional_positive_integer(value: Option<&String>, name: &str) -> Result<Option<i32>> {
    let Some(value) = value
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    let value = value
        .parse::<i32>()
        .map_err(|_| bad_request(format!("{name} must be a whole number")))?;
    if value <= 0 {
        return Err(bad_request(format!("{name} must be greater than zero")).into());
    }
    Ok(Some(value))
}

fn positive_number(value: &str, name: &str) -> Result<f64> {
    let value = value
        .trim()
        .parse::<f64>()
        .map_err(|_| bad_request(format!("{name} must be a number")))?;
    if !value.is_finite() || value <= 0.0 {
        return Err(bad_request(format!("{name} must be greater than zero")).into());
    }
    Ok(value)
}

#[topcoat::view::component]
pub async fn goal_settings(goals: &GoalSettings) -> Result {
    let effective_from = goals.effective_from.to_string();
    let default_goal = |code: &str| {
        goals
            .nutrient_goals
            .iter()
            .find(|goal| goal.code == code)
            .map(|goal| goal.target.to_string())
            .unwrap_or_default()
    };
    let extra_goals: Vec<(String, String, String)> = goals
        .nutrient_goals
        .iter()
        .filter(|goal| !DEFAULT_NUTRIENTS.contains(&goal.code.as_str()))
        .map(|goal| {
            (
                format!("nutrient_{}", goal.code),
                goal.name.clone(),
                goal.target.to_string(),
            )
        })
        .collect();
    let adjustment_options = view! {
        <option value="maintain" selected=(goals.adjustment_kind == "maintain")>"Match burn"</option>
        <option value="deficit" selected=(goals.adjustment_kind == "deficit")>"Below burn"</option>
        <option value="surplus" selected=(goals.adjustment_kind == "surplus")>"Above burn"</option>
    };

    view! {
        <section id="goals" aria-labelledby="goals-heading" class="mt-12 border-t border-gray-200 pt-6">
            <h2 id="goals-heading" class="text-base font-medium text-gray-1000">"Goals"</h2>

            <form method="post" action="/settings/goals" hx-boost="true" class="mt-4 space-y-5">
                <label class="block max-w-xs text-sm text-gray-700">
                    <span class="mb-1 block">"Starting from"</span>
                    <input name="effective_from" type="date" required="true" value=(&effective_from) class=(input::FIELD)>
                </label>

                <fieldset class="rounded-xl border border-gray-200 p-4">
                    <legend class="px-1 text-sm font-medium text-gray-900">"Daily targets"</legend>
                    <div class="grid grid-cols-2 gap-3">
                        goal_input(name: "daily_burn_kcal", label: "Baseline burn", value: goals.daily_burn_kcal.map(|value| value.to_string()).unwrap_or_default(), unit: "kcal")
                        goal_input(name: "water_goal_ml", label: "Water", value: goals.water_goal_ml.map(|value| value.to_string()).unwrap_or_default(), unit: "ml")
                    </div>
                </fieldset>

                <fieldset class="group rounded-xl border border-gray-200 p-4">
                    <legend class="px-1 text-sm font-medium text-gray-900">"Calorie target"</legend>
                    <div class="grid grid-cols-2 gap-3">
                        <label class="text-sm text-gray-700 group-has-[option[value=maintain]:checked]:col-span-2">
                            <span class="mb-1 block">"Mode"</span>
                            select_control(name: "adjustment_kind", options: adjustment_options)
                        </label>
                        <div class="group-has-[option[value=maintain]:checked]:hidden">
                            goal_input(name: "adjustment_amount", label: "By", value: goals.adjustment_amount.map(|value| value.to_string()).unwrap_or_default(), unit: "kcal")
                        </div>
                    </div>
                </fieldset>

                <fieldset class="rounded-xl border border-gray-200 p-4">
                    <legend class="px-1 text-sm font-medium text-gray-900">"Nutrition"</legend>
                    <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
                        goal_input(name: "nutrient_protein", label: "Protein", value: default_goal("protein"), unit: "g")
                        goal_input(name: "nutrient_carbohydrate", label: "Carbs", value: default_goal("carbohydrate"), unit: "g")
                        goal_input(name: "nutrient_fat", label: "Fat", value: default_goal("fat"), unit: "g")
                        goal_input(name: "nutrient_fibre", label: "Fibre", value: default_goal("fibre"), unit: "g")
                    </div>

                    if !extra_goals.is_empty() {
                        <div class="mt-3 grid grid-cols-2 gap-3 sm:grid-cols-4">
                            for goal in &extra_goals {
                                goal_input(name: &goal.0, label: &goal.1, value: goal.2.clone(), unit: "g")
                            }
                        </div>
                    }

                    <div class="mt-4 border-t border-gray-200 pt-4">
                        <p class="mb-2 text-sm font-medium text-gray-900">"Add nutrient goal"</p>
                        <div class="grid grid-cols-[minmax(0,1fr)_7rem] gap-3">
                            <label class="text-sm text-gray-700">
                                <span class="mb-1 block">"Nutrient"</span>
                                <input name="add_nutrient_name" list="nutrient-goal-names" autocomplete="off" class=(input::FIELD)>
                            </label>
                            goal_input(name: "add_nutrient_target", label: "Goal", value: String::new(), unit: "g")
                        </div>
                        <datalist id="nutrient-goal-names">
                            for name in &goals.available_nutrients {
                                <option value=(name)></option>
                            }
                        </datalist>
                    </div>
                </fieldset>

                <button type="submit" class=(button::OUTLINE)>"Save goals"</button>
            </form>
        </section>
    }
}

#[topcoat::view::component]
async fn goal_input(name: &str, label: &str, value: String, unit: &str) -> Result {
    view! {
        <label class="min-w-0 text-sm text-gray-700">
            <span class="mb-1 block truncate">(label)</span>
            <span class="relative block">
                <input name=(name) type="text" inputmode="decimal" value=(&value) class=(input::FIELD_WITH_UNIT)>
                <span class="pointer-events-none absolute inset-y-0 right-3 grid place-items-center text-xs text-gray-500">(unit)</span>
            </span>
        </label>
    }
}
