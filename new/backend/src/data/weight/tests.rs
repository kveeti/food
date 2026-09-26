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
async fn keeps_each_measurement_and_sorts_by_measurement_time(pool: PgPool) -> Result<()> {
    let (data, user, _) = setup(pool).await?;
    let morning = "2026-05-01T04:30:00Z".parse()?;
    let evening = "2026-05-01T17:00:00Z".parse()?;
    let later = data.add_weight(user, 78.5, evening).await?;
    let earlier = data.add_weight(user, 78.1, morning).await?;
    let repeated = data.add_weight(user, 78.2, morning).await?;

    let entries = data.recent_weight_entries(user, 100).await?;
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].id, later.id);
    assert_eq!(entries[1].id, repeated.id);
    assert_eq!(entries[2].id, earlier.id);

    let corrected_time = "2026-04-30T04:30:00Z".parse()?;
    data.update_weight(user, later.id, 78.25, corrected_time)
        .await?
        .unwrap();
    let entries = data.recent_weight_entries(user, 100).await?;
    assert_eq!(entries[0].id, repeated.id);
    assert_eq!(entries[2].id, later.id);
    assert_eq!(entries[2].weight_kg, 78.25);
    assert_eq!(entries[2].measured_at, corrected_time);

    assert!(data.delete_weight(user, repeated.id).await?);
    let entries = data.recent_weight_entries(user, 100).await?;
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].id, earlier.id);
    assert!(!data.delete_weight(user, repeated.id).await?);
    Ok(())
}

#[sqlx::test]
async fn limits_chart_points_and_averages_buckets(pool: PgPool) -> Result<()> {
    let (data, user, _) = setup(pool).await?;
    let first_at = "2026-05-01T04:30:00Z".parse()?;
    let same_bucket = "2026-05-01T05:30:00Z".parse()?;
    let last_at = "2026-05-02T04:30:00Z".parse()?;
    data.add_weight(user, 78.0, first_at).await?;
    data.add_weight(user, 80.0, same_bucket).await?;
    data.add_weight(user, 82.0, last_at).await?;

    let points = data
        .weight_chart_points(user, Some(first_at), Some(last_at), 2, "UTC")
        .await?;

    assert_eq!(points.len(), 2);
    assert_eq!(points[0].weight_kg, 79.0);
    assert_eq!(points[0].trend_weight_kg, 79.0);
    assert_eq!(points[1].weight_kg, 82.0);
    assert_eq!(points[1].trend_weight_kg, 80.5);
    Ok(())
}

#[sqlx::test]
async fn trend_uses_prior_local_days_without_showing_them(pool: PgPool) -> Result<()> {
    let (data, user, _) = setup(pool).await?;
    let prior = "2026-04-30T22:00:00Z".parse()?;
    let from = "2026-05-01T21:30:00Z".parse()?;
    let same_day_before_range = "2026-05-01T21:00:00Z".parse()?;
    let visible = "2026-05-01T22:00:00Z".parse()?;
    data.add_weight(user, 60.0, prior).await?;
    data.add_weight(user, 90.0, same_day_before_range).await?;
    let first = data.add_weight(user, 80.0, visible).await?;

    let points = data
        .weight_chart_points(user, Some(from), None, 200, "Europe/Helsinki")
        .await?;

    assert_eq!(points.len(), 1);
    assert_eq!(points[0].measured_at, visible);
    assert_eq!(points[0].weight_kg, 80.0);
    assert_eq!(points[0].trend_weight_kg, 72.5);
    assert_eq!(
        data.first_weight_entry_since(user, Some(from), None)
            .await?
            .unwrap()
            .id,
        first.id
    );
    Ok(())
}

#[sqlx::test]
async fn bucket_date_represents_its_center(pool: PgPool) -> Result<()> {
    let (data, user, _) = setup(pool).await?;
    let first = "2026-05-01T04:30:00Z".parse()?;
    let middle = "2026-05-02T04:30:00Z".parse()?;
    let last = "2026-05-03T04:30:00Z".parse()?;
    data.add_weight(user, 70.0, first).await?;
    data.add_weight(user, 80.0, middle).await?;
    data.add_weight(user, 90.0, last).await?;

    let points = data.weight_chart_points(user, None, None, 2, "UTC").await?;

    assert_eq!(points.len(), 2);
    let bucket_center: DateTime<Utc> = "2026-05-01T16:30:00Z".parse()?;
    assert_eq!(points[0].measured_at, bucket_center);
    assert_eq!(points[0].weight_kg, 75.0);
    assert_eq!(points[1].measured_at, last);
    assert_eq!(
        data.first_weight_entry_since(user, None, None)
            .await?
            .unwrap()
            .weight_kg,
        70.0
    );
    assert_eq!(
        data.last_weight_entry_in_range(user, None, Some(middle))
            .await?
            .unwrap()
            .weight_kg,
        80.0
    );
    Ok(())
}

#[sqlx::test]
async fn other_users_cannot_read_change_or_delete_measurements(pool: PgPool) -> Result<()> {
    let (data, user, other) = setup(pool).await?;
    let measured_at = "2026-05-01T04:30:00Z".parse()?;
    let entry = data.add_weight(user, 78.25, measured_at).await?;

    assert!(data.recent_weight_entries(other, 100).await?.is_empty());
    assert!(
        data.update_weight(other, entry.id, 90.0, measured_at)
            .await?
            .is_none()
    );
    assert!(!data.delete_weight(other, entry.id).await?);

    let entries = data.recent_weight_entries(user, 100).await?;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].weight_kg, 78.25);
    Ok(())
}

#[sqlx::test]
async fn rejects_nonpositive_and_nonfinite_weights(pool: PgPool) -> Result<()> {
    let (data, user, _) = setup(pool).await?;
    let measured_at = "2026-05-01T04:30:00Z".parse()?;
    assert!(data.add_weight(user, 0.0, measured_at).await.is_err());
    assert!(data.add_weight(user, -1.0, measured_at).await.is_err());
    assert!(data.add_weight(user, f64::NAN, measured_at).await.is_err());
    assert!(
        data.add_weight(user, f64::INFINITY, measured_at)
            .await
            .is_err()
    );
    assert!(data.recent_weight_entries(user, 100).await?.is_empty());
    Ok(())
}
