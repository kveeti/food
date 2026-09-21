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

    let entries = data.weight_entries(user).await?;
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].id, later.id);
    assert_eq!(entries[1].id, repeated.id);
    assert_eq!(entries[2].id, earlier.id);

    let corrected_time = "2026-04-30T04:30:00Z".parse()?;
    data.update_weight(user, later.id, 78.25, corrected_time)
        .await?
        .unwrap();
    let entries = data.weight_entries(user).await?;
    assert_eq!(entries[0].id, repeated.id);
    assert_eq!(entries[2].id, later.id);
    assert_eq!(entries[2].weight_kg, 78.25);
    assert_eq!(entries[2].measured_at, corrected_time);

    assert!(data.delete_weight(user, repeated.id).await?);
    let entries = data.weight_entries(user).await?;
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].id, earlier.id);
    assert!(!data.delete_weight(user, repeated.id).await?);
    Ok(())
}

#[sqlx::test]
async fn other_users_cannot_read_change_or_delete_measurements(pool: PgPool) -> Result<()> {
    let (data, user, other) = setup(pool).await?;
    let measured_at = "2026-05-01T04:30:00Z".parse()?;
    let entry = data.add_weight(user, 78.25, measured_at).await?;

    assert!(data.weight_entries(other).await?.is_empty());
    assert!(
        data.update_weight(other, entry.id, 90.0, measured_at)
            .await?
            .is_none()
    );
    assert!(!data.delete_weight(other, entry.id).await?);

    let entries = data.weight_entries(user).await?;
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
    assert!(data.weight_entries(user).await?.is_empty());
    Ok(())
}
