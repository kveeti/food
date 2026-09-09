use super::*;
use sqlx::PgPool;

async fn setup(pool: PgPool) -> Result<(Data, Uuid, Uuid)> {
    let user = Uuid::now_v7();
    let other = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO users (id, issuer, subject)
         VALUES ($1, 'test', 'alice'), ($2, 'test', 'bob')",
    )
    .bind(user)
    .bind(other)
    .execute(&pool)
    .await?;
    Ok((Data { pool }, user, other))
}

#[sqlx::test]
async fn dated_profiles_keep_full_snapshots_and_user_ownership(pool: PgPool) -> Result<()> {
    let (data, user, other) = setup(pool).await?;
    let definitions = data.nutrient_definitions().await?;
    let protein = definitions
        .iter()
        .find(|nutrient| nutrient.code == "protein")
        .unwrap();
    let sodium = definitions
        .iter()
        .find(|nutrient| nutrient.code == "sodium")
        .unwrap();
    let first_day = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
    let second_day = NaiveDate::from_ymd_opt(2026, 2, 1).unwrap();

    data.save_goal_profile(NewGoalProfile {
        user_id: user,
        starts_on: first_day,
        daily_burn_kj: Some(10_041.6),
        food_adjustment_kj: Some(-1_673.6),
        water_ml: Some(2_500),
        nutrients: &[NewNutrientGoal {
            nutrient_id: protein.id,
            value: 120.0,
        }],
    })
    .await?;
    data.save_goal_profile(NewGoalProfile {
        user_id: user,
        starts_on: second_day,
        daily_burn_kj: Some(10_041.6),
        food_adjustment_kj: Some(0.0),
        water_ml: Some(3_000),
        nutrients: &[
            NewNutrientGoal {
                nutrient_id: protein.id,
                value: 140.0,
            },
            NewNutrientGoal {
                nutrient_id: sodium.id,
                value: 2.3,
            },
        ],
    })
    .await?;

    let january_visible = data.visible_nutrient_definitions(user, first_day).await?;
    let february_visible = data.visible_nutrient_definitions(user, second_day).await?;
    assert!(
        january_visible
            .iter()
            .all(|nutrient| nutrient.show_by_default)
    );
    assert!(
        february_visible
            .iter()
            .any(|nutrient| nutrient.code == "sodium")
    );
    assert!(
        february_visible
            .iter()
            .all(|nutrient| nutrient.show_by_default || nutrient.code == "sodium")
    );

    assert!(
        data.goal_profile(user, first_day.pred_opt().unwrap())
            .await?
            .is_none()
    );
    assert!(data.goal_profile(other, second_day).await?.is_none());

    let (january, january_nutrients) = data
        .goal_profile(user, second_day.pred_opt().unwrap())
        .await?
        .unwrap();
    assert_eq!(january.starts_on, first_day);
    assert_eq!(january.water_ml, Some(2_500));
    assert_eq!(january_nutrients.len(), 1);
    assert_eq!(january_nutrients[0].code, "protein");
    assert_eq!(january_nutrients[0].value, 120.0);

    let (february, february_nutrients) = data.goal_profile(user, second_day).await?.unwrap();
    assert_eq!(february.starts_on, second_day);
    assert_eq!(february.water_ml, Some(3_000));
    assert_eq!(february_nutrients.len(), 2);
    assert_eq!(february_nutrients[0].code, "protein");
    assert_eq!(february_nutrients[0].value, 140.0);
    assert_eq!(february_nutrients[1].code, "sodium");
    assert!((february_nutrients[1].value - 2_300.0).abs() < 0.0001);

    Ok(())
}

#[sqlx::test]
async fn day_totals_use_the_users_local_day_and_keep_missing_nutrients_visible(
    pool: PgPool,
) -> Result<()> {
    let (data, user, other) = setup(pool.clone()).await?;
    let energy = sqlx::query_scalar::<_, i16>("SELECT id FROM nutrients WHERE code = 'energy'")
        .fetch_one(&pool)
        .await?;
    let protein = sqlx::query_scalar::<_, i16>("SELECT id FROM nutrients WHERE code = 'protein'")
        .fetch_one(&pool)
        .await?;
    let first = Uuid::now_v7();
    let missing_energy = Uuid::now_v7();
    let previous_day = Uuid::now_v7();
    let other_user = Uuid::now_v7();

    sqlx::query(
        "INSERT INTO food_entries (
            id, user_id, amount, unit, eaten_at, food_name
         ) VALUES
            ($1, $5, 100, 'g', '2026-01-01 22:30:00+00', 'First'),
            ($2, $5, 100, 'g', '2026-01-02 12:00:00+00', 'Missing energy'),
            ($3, $5, 100, 'g', '2025-12-31 22:30:00+00', 'Previous day'),
            ($4, $6, 100, 'g', '2026-01-02 12:00:00+00', 'Other user')",
    )
    .bind(first)
    .bind(missing_energy)
    .bind(previous_day)
    .bind(other_user)
    .bind(user)
    .bind(other)
    .execute(&pool)
    .await?;
    sqlx::query(
        "INSERT INTO water_entries (id, user_id, amount_ml, consumed_at)
         VALUES
            ($1, $5, 100, '2026-01-01 22:30:00+00'),
            ($2, $5, 200, '2026-01-02 12:00:00+00'),
            ($3, $5, 1000, '2025-12-31 22:30:00+00'),
            ($4, $6, 1000, '2026-01-02 12:00:00+00')",
    )
    .bind(Uuid::now_v7())
    .bind(Uuid::now_v7())
    .bind(Uuid::now_v7())
    .bind(Uuid::now_v7())
    .bind(user)
    .bind(other)
    .execute(&pool)
    .await?;
    sqlx::query(
        "INSERT INTO food_entry_nutrients (food_entry_id, nutrient_id, basis_value)
         VALUES
            ($1, $5, 418.4), ($1, $6, 10),
            ($2, $6, 20),
            ($3, $5, 4184),
            ($4, $5, 4184)",
    )
    .bind(first)
    .bind(missing_energy)
    .bind(previous_day)
    .bind(other_user)
    .bind(energy)
    .bind(protein)
    .execute(&pool)
    .await?;

    let totals = data
        .visible_nutrient_totals(
            user,
            NaiveDate::from_ymd_opt(2026, 1, 2).unwrap(),
            "Europe/Helsinki",
        )
        .await?;
    let energy = totals.iter().find(|total| total.code == "energy").unwrap();
    let protein = totals.iter().find(|total| total.code == "protein").unwrap();

    assert!(totals.iter().any(|total| total.code == "fat"));
    assert!(!totals.iter().any(|total| total.code == "sodium"));
    assert!((energy.value - 100.0).abs() < 0.0001);
    assert_eq!(energy.known_entries, 1);
    assert_eq!(energy.total_entries, 2);
    assert_eq!(protein.value, 30.0);
    assert_eq!(protein.known_entries, 2);
    assert_eq!(protein.total_entries, 2);
    assert_eq!(
        data.water_total(
            user,
            NaiveDate::from_ymd_opt(2026, 1, 2).unwrap(),
            "Europe/Helsinki",
        )
        .await?,
        300
    );

    Ok(())
}
