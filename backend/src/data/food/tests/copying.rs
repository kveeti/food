use super::*;

#[sqlx::test]
async fn copies_selected_snapshots_with_new_amounts_and_local_time(pool: PgPool) -> Result<()> {
    let (data, user, _) = setup(pool).await?;
    let day = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
    let apple = log_food(&data, user, day, MealChoice::Breakfast).await?;
    let source = apple.meal_id.as_deref().unwrap().parse()?;
    let juice = data
        .add_food(
            user,
            NewFoodEntry {
                meal: MealChoice::ContinuePrevious,
                meal_id: Some(source),
                food_id: "00000000-0000-4000-8000-000000000002".parse()?,
                amount: 200.0,
                unit: "ml",
                date: day,
                timezone: "UTC",
            },
        )
        .await?
        .unwrap();
    let omitted = log_food(&data, user, day, MealChoice::ContinuePrevious).await?;
    sqlx::query("DELETE FROM foods WHERE id = $1")
        .bind(apple.food_id.as_deref().unwrap().parse::<Uuid>()?)
        .execute(&data.pool)
        .await?;
    sqlx::query("UPDATE foods SET display_name = 'Changed', basis_unit = 'g', is_archived = true")
        .execute(&data.pool)
        .await?;
    sqlx::query("UPDATE food_nutrients SET value = 0")
        .execute(&data.pool)
        .await?;

    let target_day = day.succ_opt().unwrap();
    let copied_id = data
        .copy_meal(
            user,
            source,
            CopyMealInput {
                date: target_day,
                target: CopyMealTarget::New {
                    meal: MealChoice::Breakfast,
                    time: NaiveTime::from_hms_opt(0, 30, 0).unwrap(),
                },
                entries: vec![
                    CopyMealEntry {
                        entry_id: juice.id.parse()?,
                        amount: 75.5,
                    },
                    CopyMealEntry {
                        entry_id: apple.id.parse()?,
                        amount: 150.0,
                    },
                ],
            },
            "Europe/Helsinki",
        )
        .await?
        .unwrap();
    assert_ne!(copied_id, source);
    let meals = data.food_meals(user, target_day, "Europe/Helsinki").await?;
    assert_eq!(meals.len(), 1);
    let copy = &meals[0];
    assert_eq!(copy.id.as_deref(), Some(copied_id.to_string().as_str()));
    assert_eq!(copy.name.as_deref(), Some("Breakfast"));
    assert_eq!(copy.started_at.to_rfc3339(), "2026-01-10T22:30:00+00:00");
    assert_eq!(copy.entries.len(), 2);
    assert_eq!(copy.entries[0].food_name, "Apple juice");
    assert_eq!(copy.entries[0].food_brand.as_deref(), Some("Orchard"));
    assert_eq!(copy.entries[0].unit, "ml");
    assert_eq!(copy.entries[0].amount, 75.5);
    assert_eq!(copy.entries[1].food_name, "Apple");
    assert_eq!(copy.entries[1].food_id, None);
    assert_eq!(copy.entries[1].amount, 150.0);
    for (copied, original) in copy.entries.iter().zip([&juice, &apple]) {
        assert_ne!(copied.id, original.id);
        assert_eq!(copied.eaten_at, copy.started_at);
        let nutrients = copied.nutrients.as_array().unwrap();
        assert_eq!(nutrients.len(), 2);
        let energy = nutrients.iter().find(|n| n["code"] == "energy").unwrap()["value"]
            .as_f64()
            .unwrap();
        assert!((energy - copied.amount * 0.5).abs() < 0.0001);
        assert!(!nutrients.iter().any(|n| n["code"] == "fat"));
    }
    data.update_food_entry(user, copy.entries[1].id.parse()?, 300.0)
        .await?;
    let original = data.food_meals(user, day, "UTC").await?;
    let original = original
        .iter()
        .find(|meal| meal.id == apple.meal_id)
        .unwrap();
    assert_eq!(original.entries.len(), 3);
    assert!(original.entries.iter().any(|entry| entry.id == omitted.id));
    assert_eq!(
        original
            .entries
            .iter()
            .find(|entry| entry.id == apple.id)
            .unwrap()
            .amount,
        100.0
    );
    Ok(())
}

#[sqlx::test]
async fn copying_into_existing_meals_uses_their_time_including_the_source_meal(
    pool: PgPool,
) -> Result<()> {
    let (data, user, _) = setup(pool).await?;
    let day = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
    let source_entry = log_food(&data, user, day, MealChoice::Breakfast).await?;
    let source = source_entry.meal_id.as_deref().unwrap().parse()?;
    let target_day = day.succ_opt().unwrap();
    let target_entry = log_food(&data, user, target_day, MealChoice::Lunch).await?;
    let target = target_entry.meal_id.as_deref().unwrap().parse()?;
    let copied_id = data
        .copy_meal(
            user,
            source,
            CopyMealInput {
                date: target_day,
                target: CopyMealTarget::Existing { meal_id: target },
                entries: vec![CopyMealEntry {
                    entry_id: source_entry.id.parse()?,
                    amount: 200.0,
                }],
            },
            "UTC",
        )
        .await?
        .unwrap();
    assert_eq!(copied_id, target);
    let meals = data.food_meals(user, target_day, "UTC").await?;
    assert_eq!(meals.len(), 1);
    assert_eq!(meals[0].name.as_deref(), Some("Lunch"));
    assert_eq!(meals[0].entries.len(), 2);
    assert!(
        meals[0]
            .entries
            .iter()
            .all(|entry| entry.eaten_at == target_entry.eaten_at)
    );

    data.copy_meal(
        user,
        source,
        CopyMealInput {
            date: day,
            target: CopyMealTarget::Existing { meal_id: source },
            entries: vec![CopyMealEntry {
                entry_id: source_entry.id.parse()?,
                amount: 50.0,
            }],
        },
        "UTC",
    )
    .await?
    .unwrap();
    let meals = data.food_meals(user, day, "UTC").await?;
    assert_eq!(meals.len(), 1);
    assert_eq!(meals[0].entries.len(), 2);
    assert_eq!(meals[0].entries[0].amount, 50.0);
    assert_eq!(meals[0].entries[1].amount, 100.0);
    Ok(())
}

#[sqlx::test]
async fn rejects_foreign_missing_duplicate_and_invalid_selections_without_partial_copies(
    pool: PgPool,
) -> Result<()> {
    let (data, user, other) = setup(pool).await?;
    let day = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
    let source_entry = log_food(&data, user, day, MealChoice::Breakfast).await?;
    let source = source_entry.meal_id.as_deref().unwrap().parse()?;
    let entry_id = source_entry.id.parse()?;
    let other_entry = log_food(&data, other, day, MealChoice::Lunch).await?;
    let other_source = other_entry.meal_id.as_deref().unwrap().parse()?;
    let other_entry_id = other_entry.id.parse()?;
    let target_day = day.succ_opt().unwrap();
    let new_target = || CopyMealTarget::New {
        meal: MealChoice::Breakfast,
        time: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
    };
    for (owner, meal, entries, target) in [
        (other, source, vec![(entry_id, 100.0)], new_target()),
        (
            user,
            other_source,
            vec![(other_entry_id, 100.0)],
            new_target(),
        ),
        (
            user,
            source,
            vec![(entry_id, 100.0), (other_entry_id, 100.0)],
            new_target(),
        ),
        (
            user,
            source,
            vec![(entry_id, 100.0), (Uuid::now_v7(), 100.0)],
            new_target(),
        ),
        (
            user,
            source,
            vec![(entry_id, 100.0), (entry_id, 100.0)],
            new_target(),
        ),
        (user, source, vec![], new_target()),
        (user, source, vec![(entry_id, 0.0)], new_target()),
        (user, source, vec![(entry_id, 100_001.0)], new_target()),
        (user, source, vec![(entry_id, f64::NAN)], new_target()),
        (
            user,
            source,
            vec![(entry_id, 100.0)],
            CopyMealTarget::Existing {
                meal_id: other_source,
            },
        ),
        (
            user,
            source,
            vec![(entry_id, 100.0)],
            CopyMealTarget::Existing { meal_id: source },
        ),
        (
            user,
            source,
            vec![(entry_id, 100.0)],
            CopyMealTarget::Existing {
                meal_id: Uuid::now_v7(),
            },
        ),
    ] {
        let result = data
            .copy_meal(
                owner,
                meal,
                CopyMealInput {
                    date: target_day,
                    target,
                    entries: entries
                        .into_iter()
                        .map(|(entry_id, amount)| CopyMealEntry { entry_id, amount })
                        .collect(),
                },
                "UTC",
            )
            .await?;
        assert!(result.is_none());
    }
    let counts: (i64, i64) =
        sqlx::query_as("SELECT (SELECT count(*) FROM meals), (SELECT count(*) FROM food_entries)")
            .fetch_one(&data.pool)
            .await?;
    assert_eq!(counts, (2, 2));
    Ok(())
}
