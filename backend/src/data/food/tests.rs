use super::*;
use sqlx::PgPool;

mod copying;
mod deleting;

async fn setup(pool: PgPool) -> Result<(Data, Uuid, Uuid)> {
    sqlx::raw_sql(include_str!("../../../test-support/catalog.sql"))
        .execute(&pool)
        .await?;
    let user = Uuid::now_v7();
    let other = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO users (id, issuer, subject) VALUES ($1, 'test', 'alice'), ($2, 'test', 'bob')",
    )
    .bind(user)
    .bind(other)
    .execute(&pool)
    .await?;
    Ok((Data { pool }, user, other))
}

#[sqlx::test]
async fn snapshots_survive_catalog_changes(pool: PgPool) -> Result<()> {
    let (data, user, _) = setup(pool).await?;
    let food = data.search_foods(user, "omena").await?.remove(0);
    let food_id = food.id.parse()?;
    let day = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
    let entry = data
        .add_food(
            user,
            NewFoodEntry {
                meal: MealChoice::Breakfast,
                meal_id: None,
                food_id,
                amount: 150.5,
                unit: "g",
                date: day,
                timezone: "Europe/Helsinki",
            },
        )
        .await?
        .unwrap();
    sqlx::query("UPDATE foods SET display_name = 'Changed', is_archived = true WHERE id = $1")
        .bind(food_id)
        .execute(&data.pool)
        .await?;
    sqlx::query("UPDATE food_nutrients SET value = 0 WHERE food_id = $1")
        .bind(food_id)
        .execute(&data.pool)
        .await?;
    let meals = data.food_meals(user, day, "Europe/Helsinki").await?;
    assert_eq!(meals.len(), 1);
    assert_eq!(meals[0].entries.len(), 1);
    assert_eq!(meals[0].entries[0].food_name, "Apple");
    assert_eq!(meals[0].entries[0].nutrients, entry.nutrients);
    let nutrients = entry.nutrients.as_array().unwrap();
    let energy = nutrients.iter().find(|n| n["code"] == "energy").unwrap()["value"]
        .as_f64()
        .unwrap();
    assert!((energy - 75.25).abs() < 0.0001);
    assert!(!nutrients.iter().any(|n| n["code"] == "fat"));
    assert!(
        data.food_meals(user, day.succ_opt().unwrap(), "Europe/Helsinki")
            .await?
            .is_empty()
    );
    assert!(data.food(user, food_id).await?.is_none());
    assert!(
        data.search_foods(user, "apple")
            .await?
            .iter()
            .all(|f| f.id != food.id)
    );
    assert!(
        data.add_food(
            user,
            NewFoodEntry {
                meal: MealChoice::Breakfast,
                meal_id: None,
                food_id,
                amount: 100.0,
                unit: "g",
                date: day,
                timezone: "UTC",
            }
        )
        .await?
        .is_none()
    );
    Ok(())
}

#[sqlx::test]
async fn private_foods_and_entries_are_scoped_to_their_owner(pool: PgPool) -> Result<()> {
    let (data, user, other) = setup(pool).await?;
    let food_id = Uuid::now_v7();
    sqlx::query("INSERT INTO foods (id, owner_user_id, display_name, basis_unit) VALUES ($1, $2, 'Private apple', 'g')")
        .bind(food_id).bind(other).execute(&data.pool).await?;
    sqlx::query("SELECT refresh_food_search_vector($1)")
        .bind(food_id)
        .execute(&data.pool)
        .await?;
    assert!(data.search_foods(user, "private").await?.is_empty());
    assert_eq!(data.search_foods(other, "private").await?.len(), 1);
    assert!(data.food(user, food_id).await?.is_none());
    assert!(data.food(other, food_id).await?.is_some());
    let date = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
    for (owner, unit) in [(user, "g"), (other, "ml")] {
        assert!(
            data.add_food(
                owner,
                NewFoodEntry {
                    meal: MealChoice::Breakfast,
                    meal_id: None,
                    food_id,
                    amount: 100.0,
                    unit,
                    date,
                    timezone: "UTC",
                }
            )
            .await?
            .is_none()
        );
    }
    let entry = data
        .add_food(
            other,
            NewFoodEntry {
                meal: MealChoice::Breakfast,
                meal_id: None,
                food_id,
                amount: 100.0,
                unit: "g",
                date,
                timezone: "UTC",
            },
        )
        .await?
        .unwrap();
    let id = entry.id.parse()?;
    assert!(data.food_meals(user, date, "UTC").await?.is_empty());
    assert!(!data.delete_food_entry(user, id).await?);
    assert_eq!(data.food_meals(other, date, "UTC").await?.len(), 1);
    assert!(data.delete_food_entry(other, id).await?);
    Ok(())
}

async fn log_food(data: &Data, user: Uuid, day: NaiveDate, meal: MealChoice) -> Result<FoodEntry> {
    let food = data.search_foods(user, "omena").await?.remove(0);
    let meal_id = if matches!(meal, MealChoice::ContinuePrevious) {
        data.meal_suggestion(user, day, "UTC")
            .await?
            .previous_meal_id
            .map(|id| id.parse())
            .transpose()?
    } else {
        None
    };
    Ok(data
        .add_food(
            user,
            NewFoodEntry {
                meal,
                meal_id,
                food_id: food.id.parse()?,
                amount: 100.0,
                unit: "g",
                date: day,
                timezone: "UTC",
            },
        )
        .await?
        .unwrap())
}

#[sqlx::test]
async fn meals_continue_edit_and_delete_with_owner_isolation(pool: PgPool) -> Result<()> {
    let (data, user, other) = setup(pool).await?;
    let day = Utc::now().date_naive();
    let first = log_food(&data, user, day, MealChoice::Lunch).await?;
    let second = log_food(&data, user, day, MealChoice::ContinuePrevious).await?;
    assert_eq!(first.meal_id, second.meal_id);
    assert_eq!(second.meal_name.as_deref(), Some("Lunch"));
    assert_ne!(first.id, second.id);
    let dinner = log_food(&data, user, day, MealChoice::Dinner).await?;
    let food = data.search_foods(user, "omena").await?.remove(0);
    let stale_continue = data
        .add_food(
            user,
            NewFoodEntry {
                meal: MealChoice::ContinuePrevious,
                meal_id: Some(first.meal_id.as_deref().unwrap().parse()?),
                food_id: food.id.parse()?,
                amount: 100.0,
                unit: "g",
                date: day,
                timezone: "UTC",
            },
        )
        .await?;
    assert!(stale_continue.is_none());
    let continued = log_food(&data, user, day, MealChoice::ContinuePrevious).await?;
    assert_eq!(dinner.meal_id, continued.meal_id);
    assert_ne!(first.meal_id, dinner.meal_id);
    let id = first.id.parse()?;
    assert!(data.update_food_entry(other, id, 200.0).await?.is_none());
    let updated = data.update_food_entry(user, id, 200.0).await?.unwrap();
    assert_eq!(updated.amount, 200.0);
    assert_eq!(updated.meal_id, first.meal_id);
    let original_energy = first
        .nutrients
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["code"] == "energy")
        .unwrap()["value"]
        .as_f64()
        .unwrap();
    let updated_energy = updated
        .nutrients
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["code"] == "energy")
        .unwrap()["value"]
        .as_f64()
        .unwrap();
    assert!((updated_energy - original_energy * 2.0).abs() < 0.0001);
    assert!(!data.delete_food_entry(other, id).await?);
    assert!(data.delete_food_entry(user, id).await?);
    assert!(data.delete_food_entry(user, second.id.parse()?).await?);
    let remains: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM meals WHERE id = $1)")
        .bind(first.meal_id.unwrap().parse::<Uuid>()?)
        .fetch_one(&data.pool)
        .await?;
    assert!(!remains);
    assert!(
        data.meal_suggestion(other, day, "UTC")
            .await?
            .previous_meal_id
            .is_none()
    );
    Ok(())
}

#[sqlx::test]
async fn suggestions_use_recent_meals_and_history_without_clock_defaults(
    pool: PgPool,
) -> Result<()> {
    let (data, user, other) = setup(pool).await?;
    let day = Utc::now().date_naive();
    assert!(data.meal_suggestion(user, day, "UTC").await?.meal.is_none());
    log_food(
        &data,
        user,
        day - chrono::Duration::days(7),
        MealChoice::Breakfast,
    )
    .await?;
    log_food(
        &data,
        user,
        day - chrono::Duration::days(1),
        MealChoice::Lunch,
    )
    .await?;
    assert_eq!(
        data.meal_suggestion(user, day, "UTC")
            .await?
            .meal
            .as_deref(),
        Some("breakfast")
    );
    assert!(
        data.meal_suggestion(other, day, "UTC")
            .await?
            .meal
            .is_none()
    );
    let recent = log_food(&data, user, day, MealChoice::Snack).await?;
    let suggestion = data.meal_suggestion(user, day, "UTC").await?;
    assert_eq!(suggestion.meal.as_deref(), Some("continue_previous"));
    assert_eq!(suggestion.previous_meal_id, recent.meal_id);
    assert_eq!(suggestion.previous_meal_name.as_deref(), Some("Snack"));
    Ok(())
}

#[sqlx::test]
async fn diary_returns_distinct_meals_with_ordered_entries_and_keeps_unassigned_food(
    pool: PgPool,
) -> Result<()> {
    let (data, user, other) = setup(pool).await?;
    let day = Utc::now().date_naive();
    let first = log_food(&data, user, day, MealChoice::Lunch).await?;
    let second = log_food(&data, user, day, MealChoice::ContinuePrevious).await?;
    let third = log_food(&data, user, day, MealChoice::Lunch).await?;
    let fourth = log_food(&data, user, day, MealChoice::ContinuePrevious).await?;
    log_food(&data, user, day.succ_opt().unwrap(), MealChoice::Breakfast).await?;

    let meals = data.food_meals(user, day, "UTC").await?;
    assert_eq!(meals.len(), 2);
    assert_eq!(meals[0].id, third.meal_id);
    assert_eq!(meals[1].id, first.meal_id);
    assert_eq!(meals[0].name.as_deref(), Some("Lunch"));
    assert_eq!(meals[1].name.as_deref(), Some("Lunch"));
    assert_eq!(
        meals[0]
            .entries
            .iter()
            .map(|entry| &entry.id)
            .collect::<Vec<_>>(),
        vec![&fourth.id, &third.id]
    );
    assert_eq!(
        meals[1]
            .entries
            .iter()
            .map(|entry| &entry.id)
            .collect::<Vec<_>>(),
        vec![&second.id, &first.id]
    );
    assert_eq!(meals[1].started_at, first.eaten_at);
    assert!(data.food_meals(other, day, "UTC").await?.is_empty());

    sqlx::query("DELETE FROM meals WHERE id = $1")
        .bind(first.meal_id.unwrap().parse::<Uuid>()?)
        .execute(&data.pool)
        .await?;
    let meals = data.food_meals(user, day, "UTC").await?;
    assert_eq!(meals.len(), 3);
    let unassigned = meals
        .iter()
        .filter(|meal| meal.id.is_none())
        .collect::<Vec<_>>();
    assert_eq!(unassigned.len(), 2);
    assert!(
        unassigned
            .iter()
            .all(|meal| meal.name.is_none() && meal.entries.len() == 1)
    );
    assert_eq!(
        meals.iter().map(|meal| meal.entries.len()).sum::<usize>(),
        4
    );
    Ok(())
}
