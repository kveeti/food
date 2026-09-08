use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{
        RouterBuilder,
        content::Json,
        error::{bad_request, not_found},
        path_param, query_params, route,
    },
};
use uuid::Uuid;

use super::water::{date, timezone};
use crate::{
    auth,
    data::{
        Data,
        food::{Food, FoodDetail, FoodEntry, FoodMeal, MealChoice, MealSuggestion, NewFoodEntry},
    },
};

#[topcoat::router::query_params(error = bad_request)]
struct SearchQuery {
    q: String,
}

#[topcoat::router::query_params(error = bad_request)]
struct DayQuery {
    date: String,
}

#[derive(Deserialize)]
struct AddInput {
    food_id: String,
    meal: MealChoice,
    meal_id: Option<String>,
    amount: f64,
    unit: String,
    date: String,
}

#[derive(Deserialize)]
struct UpdateInput {
    amount: f64,
}

path_param!(food_id: Uuid, error = bad_request);
path_param!(entry_id: Uuid, error = bad_request);

pub fn register(builder: RouterBuilder) -> RouterBuilder {
    builder
        .route(search)
        .route(detail)
        .route(meals)
        .route(meal_suggestion)
        .route(add)
        .route(update)
        .route(delete)
}

#[route(GET "/api/foods")]
async fn search(cx: &Cx) -> Result<Json<Vec<Food>>> {
    let user = auth::require_user(cx).await?;
    let query = query_params::<SearchQuery>(cx)?;
    let q = query.q.trim();
    if q.chars().count() > 200 {
        return Err(bad_request("search must be at most 200 characters").into());
    }
    if q.is_empty() {
        return Ok(Json(vec![]));
    }
    Ok(Json(
        app_context::<Data>(cx).search_foods(user.id, q).await?,
    ))
}

#[route(GET "/api/foods/{food_id}")]
async fn detail(cx: &Cx) -> Result<Json<FoodDetail>> {
    let user = auth::require_user(cx).await?;
    let id = path_param::<FoodId>(cx)?;
    let food = app_context::<Data>(cx)
        .food(user.id, *id)
        .await?
        .ok_or_else(not_found)?;
    Ok(Json(food))
}

#[route(GET "/api/meals")]
async fn meals(cx: &Cx) -> Result<Json<Vec<FoodMeal>>> {
    let user = auth::require_user(cx).await?;
    let query = query_params::<DayQuery>(cx)?;
    Ok(Json(
        app_context::<Data>(cx)
            .food_meals(user.id, date(&query.date)?, timezone(&user)?.name())
            .await?,
    ))
}

#[route(POST "/api/food-entries")]
async fn add(cx: &Cx, Json(input): Json<AddInput>) -> Result<Json<FoodEntry>> {
    let user = auth::require_user(cx).await?;
    if !input.amount.is_finite() || input.amount <= 0.0 || input.amount > 100_000.0 {
        return Err(bad_request("amount must be greater than 0 and at most 100000").into());
    }
    let entry = app_context::<Data>(cx)
        .add_food(
            user.id,
            NewFoodEntry {
                meal: input.meal,
                meal_id: input
                    .meal_id
                    .map(|id| {
                        id.parse()
                            .map_err(|_| bad_request("meal ID must be a UUID"))
                    })
                    .transpose()?,
                food_id: input
                    .food_id
                    .parse()
                    .map_err(|_| bad_request("food ID must be a UUID"))?,
                amount: input.amount,
                unit: &input.unit,
                date: date(&input.date)?,
                timezone: timezone(&user)?.name(),
            },
        )
        .await?
        .ok_or_else(|| {
            bad_request("food or previous meal is unavailable; select the food and meal again")
        })?;
    Ok(Json(entry))
}

#[route(GET "/api/meal-suggestion")]
async fn meal_suggestion(cx: &Cx) -> Result<Json<MealSuggestion>> {
    let user = auth::require_user(cx).await?;
    let query = query_params::<DayQuery>(cx)?;
    Ok(Json(
        app_context::<Data>(cx)
            .meal_suggestion(user.id, date(&query.date)?, timezone(&user)?.name())
            .await?,
    ))
}

#[route(PATCH "/api/food-entries/{entry_id}")]
async fn update(cx: &Cx, Json(input): Json<UpdateInput>) -> Result<Json<FoodEntry>> {
    let user = auth::require_user(cx).await?;
    if !input.amount.is_finite() || input.amount <= 0.0 || input.amount > 100_000.0 {
        return Err(bad_request("amount must be greater than 0 and at most 100000").into());
    }
    let id = path_param::<EntryId>(cx)?;
    Ok(Json(
        app_context::<Data>(cx)
            .update_food_entry(user.id, *id, input.amount)
            .await?
            .ok_or_else(not_found)?,
    ))
}

#[route(DELETE "/api/food-entries/{entry_id}")]
async fn delete(cx: &Cx) -> Result<()> {
    let user = auth::require_user(cx).await?;
    let id = path_param::<EntryId>(cx)?;
    if !app_context::<Data>(cx)
        .delete_food_entry(user.id, *id)
        .await?
    {
        return Err(not_found().into());
    }
    Ok(())
}
