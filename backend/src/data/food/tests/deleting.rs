use super::*;

#[sqlx::test]
async fn deletes_owned_meal_entries_and_snapshots_without_touching_other_meals(
    pool: PgPool,
) -> Result<()> {
    let (data, user, other) = setup(pool).await?;
    let day = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
    let first = log_food(&data, user, day, MealChoice::Breakfast).await?;
    let second = log_food(&data, user, day, MealChoice::ContinuePrevious).await?;
    let lunch = log_food(&data, user, day, MealChoice::Lunch).await?;
    let other_entry = log_food(&data, other, day, MealChoice::Dinner).await?;
    let meal_id = first.meal_id.as_deref().unwrap().parse()?;
    let entry_ids: Vec<Uuid> = vec![first.id.parse()?, second.id.parse()?];

    assert!(!data.delete_meal(other, meal_id).await?);
    assert!(!data.delete_meal(user, Uuid::now_v7()).await?);
    let meals = data.food_meals(user, day, "UTC").await?;
    assert_eq!(meals.len(), 2);
    assert_eq!(
        meals
            .iter()
            .find(|meal| meal.id == first.meal_id)
            .unwrap()
            .entries
            .len(),
        2
    );

    assert!(data.delete_meal(user, meal_id).await?);
    assert!(!data.delete_meal(user, meal_id).await?);
    let meals = data.food_meals(user, day, "UTC").await?;
    assert_eq!(meals.len(), 1);
    assert_eq!(meals[0].id, lunch.meal_id);
    assert_eq!(meals[0].entries.len(), 1);
    assert_eq!(meals[0].entries[0].nutrients, lunch.nutrients);
    let remaining: (i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM meals WHERE id = $1),
            (SELECT count(*) FROM food_entries WHERE id = ANY($2)),
            (SELECT count(*) FROM food_entry_nutrients WHERE food_entry_id = ANY($2))",
    )
    .bind(meal_id)
    .bind(&entry_ids)
    .fetch_one(&data.pool)
    .await?;
    assert_eq!(remaining, (0, 0, 0));
    let other_meals = data.food_meals(other, day, "UTC").await?;
    assert_eq!(other_meals.len(), 1);
    assert_eq!(other_meals[0].entries[0].id, other_entry.id);
    assert_eq!(other_meals[0].entries[0].nutrients, other_entry.nutrients);
    assert!(
        data.food(user, first.food_id.as_deref().unwrap().parse()?)
            .await?
            .is_some()
    );
    Ok(())
}
