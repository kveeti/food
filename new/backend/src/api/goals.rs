use std::collections::{HashMap, HashSet};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{RouterBuilder, content::Json, error::bad_request, query_params, route},
};

use crate::{
    auth,
    data::{
        Data,
        goals::{NewGoalProfile, NewNutrientGoal, NutrientDefinition, NutrientTotal},
    },
};

use super::water::{date, timezone};

const KJ_PER_KCAL: f64 = 4.184;

#[topcoat::router::query_params(error = bad_request)]
struct GoalQuery {
    date: String,
    include: Option<String>,
}

#[derive(Serialize)]
struct Nutrient {
    code: String,
    name: String,
    unit: String,
    goal: Option<f64>,
    show_by_default: bool,
}

#[derive(Serialize)]
struct Total {
    eaten: f64,
    incomplete: bool,
    unknown: bool,
}

#[derive(Serialize)]
struct NutrientProgress {
    code: String,
    #[serde(flatten)]
    total: Total,
}

#[derive(Serialize)]
struct Progress {
    calories: Total,
    water_ml: i64,
    nutrients: Vec<NutrientProgress>,
}

struct DayTotals {
    nutrients: Vec<NutrientTotal>,
    water_ml: i64,
}

#[derive(Serialize)]
struct Goals {
    starts_on: Option<NaiveDate>,
    daily_burn_kcal: Option<f64>,
    food_adjustment_kcal: Option<f64>,
    calorie_goal_kcal: Option<f64>,
    water_ml: Option<i32>,
    nutrients: Vec<Nutrient>,
    #[serde(skip_serializing_if = "Option::is_none")]
    progress: Option<Progress>,
}

#[derive(Deserialize)]
struct SaveGoalsInput {
    starts_on: String,
    daily_burn_kcal: Option<f64>,
    food_adjustment_kcal: Option<f64>,
    water_ml: Option<i32>,
    nutrients: Vec<SaveNutrientInput>,
}

#[derive(Deserialize)]
struct SaveNutrientInput {
    code: String,
    value: f64,
}

pub fn register(builder: RouterBuilder) -> RouterBuilder {
    builder.route(get).route(save)
}

#[route(GET "/api/goals")]
#[tracing::instrument(name = "api::goals::get", level = "debug", skip_all)]
async fn get(cx: &Cx) -> Result<Json<Goals>> {
    let user = auth::require_user(cx).await?;
    let query = query_params::<GoalQuery>(cx)?;
    let date = date(&query.date)?;
    let include_progress = match query.include.as_deref() {
        None => false,
        Some("progress") => true,
        Some(_) => return Err(bad_request("include must be progress").into()),
    };
    let timezone = if include_progress {
        Some(timezone(&user)?)
    } else {
        None
    };
    let nutrient_totals = async {
        match timezone {
            Some(timezone) => data(cx)
                .visible_nutrient_totals(user.id, date, timezone.name())
                .await
                .map(Some),
            None => Ok(None),
        }
    };
    let water_total = async {
        match timezone {
            Some(timezone) => data(cx)
                .water_total(user.id, date, timezone.name())
                .await
                .map(Some),
            None => Ok(None),
        }
    };
    let definitions = async {
        if include_progress {
            data(cx).visible_nutrient_definitions(user.id, date).await
        } else {
            data(cx).nutrient_definitions().await
        }
    };
    let (definitions, profile, nutrient_totals, water_total) = tokio::try_join!(
        definitions,
        data(cx).goal_profile(user.id, date),
        nutrient_totals,
        water_total,
    )?;
    let totals = nutrient_totals
        .zip(water_total)
        .map(|(nutrients, water_ml)| DayTotals {
            nutrients,
            water_ml,
        });

    Ok(Json(response(definitions, profile, totals)))
}

#[route(PUT "/api/goals")]
#[tracing::instrument(name = "api::goals::save", level = "debug", skip_all)]
async fn save(cx: &Cx, Json(input): Json<SaveGoalsInput>) -> Result<()> {
    let user = auth::require_user(cx).await?;
    let starts_on = date(&input.starts_on)?;
    validate_energy(input.daily_burn_kcal, input.food_adjustment_kcal)?;
    if input
        .water_ml
        .is_some_and(|value| !(10..=100_000).contains(&value))
    {
        return Err(bad_request("water goal must be between 10 and 100000 ml").into());
    }

    let definitions = data(cx).nutrient_definitions().await?;
    let definitions_by_code: HashMap<_, _> = definitions
        .iter()
        .map(|definition| (definition.code.as_str(), definition))
        .collect();
    let mut seen = HashSet::new();
    let mut nutrients = Vec::with_capacity(input.nutrients.len());
    for goal in &input.nutrients {
        if !goal.value.is_finite() || goal.value <= 0.0 {
            return Err(bad_request("nutrient goals must be greater than 0").into());
        }
        if !seen.insert(goal.code.as_str()) {
            return Err(bad_request("each nutrient can have only one goal").into());
        }
        let Some(definition) = definitions_by_code.get(goal.code.as_str()) else {
            return Err(bad_request("unknown nutrient").into());
        };
        nutrients.push(NewNutrientGoal {
            nutrient_id: definition.id,
            value: goal.value / definition.display_scale,
        });
    }

    data(cx)
        .save_goal_profile(NewGoalProfile {
            user_id: user.id,
            starts_on,
            daily_burn_kj: input.daily_burn_kcal.map(|value| value * KJ_PER_KCAL),
            food_adjustment_kj: input.food_adjustment_kcal.map(|value| value * KJ_PER_KCAL),
            water_ml: input.water_ml,
            nutrients: &nutrients,
        })
        .await?;

    Ok(())
}

fn validate_energy(daily_burn: Option<f64>, adjustment: Option<f64>) -> Result<()> {
    match (daily_burn, adjustment) {
        (None, None) => Ok(()),
        (Some(burn), Some(adjustment))
            if burn.is_finite()
                && adjustment.is_finite()
                && burn > 0.0
                && burn + adjustment > 0.0 =>
        {
            Ok(())
        }
        _ => Err(
            bad_request("daily burn and adjustment must form a calorie goal greater than 0").into(),
        ),
    }
}

fn response(
    definitions: Vec<NutrientDefinition>,
    profile: Option<(
        crate::data::goals::GoalProfile,
        Vec<crate::data::goals::NutrientGoal>,
    )>,
    totals: Option<DayTotals>,
) -> Goals {
    let progress = totals.map(|totals| progress(&definitions, totals));
    let Some((profile, goals)) = profile else {
        return Goals {
            starts_on: None,
            daily_burn_kcal: None,
            food_adjustment_kcal: None,
            calorie_goal_kcal: None,
            water_ml: None,
            nutrients: definitions
                .into_iter()
                .map(|definition| Nutrient {
                    code: definition.code,
                    name: definition.name,
                    unit: definition.unit,
                    goal: None,
                    show_by_default: definition.show_by_default,
                })
                .collect(),
            progress,
        };
    };
    let values: HashMap<_, _> = goals
        .into_iter()
        .map(|goal| (goal.code, goal.value))
        .collect();
    let daily_burn_kcal = profile.daily_burn_kj.map(|value| value / KJ_PER_KCAL);
    let food_adjustment_kcal = profile.food_adjustment_kj.map(|value| value / KJ_PER_KCAL);

    Goals {
        starts_on: Some(profile.starts_on),
        daily_burn_kcal,
        food_adjustment_kcal,
        calorie_goal_kcal: daily_burn_kcal
            .zip(food_adjustment_kcal)
            .map(|(burn, adjustment)| burn + adjustment),
        water_ml: profile.water_ml,
        nutrients: definitions
            .into_iter()
            .map(|definition| Nutrient {
                goal: values.get(&definition.code).copied(),
                code: definition.code,
                name: definition.name,
                unit: definition.unit,
                show_by_default: definition.show_by_default,
            })
            .collect(),
        progress,
    }
}

fn progress(definitions: &[NutrientDefinition], totals: DayTotals) -> Progress {
    let water_ml = totals.water_ml;
    let totals: HashMap<_, _> = totals
        .nutrients
        .into_iter()
        .map(|total| (total.code.clone(), total))
        .collect();
    let total = |code: &str| {
        totals.get(code).map_or(
            Total {
                eaten: 0.0,
                incomplete: false,
                unknown: false,
            },
            |total| Total {
                eaten: total.value,
                incomplete: total.known_entries < total.total_entries,
                unknown: total.total_entries > 0 && total.known_entries == 0,
            },
        )
    };

    Progress {
        calories: total("energy"),
        water_ml,
        nutrients: definitions
            .iter()
            .map(|definition| NutrientProgress {
                code: definition.code.clone(),
                total: total(&definition.code),
            })
            .collect(),
    }
}

fn data(cx: &Cx) -> &Data {
    app_context(cx)
}
