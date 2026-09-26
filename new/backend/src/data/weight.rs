use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use super::Data;

#[derive(sqlx::FromRow, Serialize)]
pub struct WeightEntry {
    pub id: Uuid,
    pub weight_kg: f64,
    pub measured_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow, Serialize)]
pub struct WeightPoint {
    pub weight_kg: f64,
    pub measured_at: DateTime<Utc>,
    pub trend_weight_kg: f64,
}

impl Data {
    #[tracing::instrument(name = "data::recent_weight_entries", level = "info", skip_all)]
    pub async fn recent_weight_entries(
        &self,
        user_id: Uuid,
        limit: i64,
    ) -> Result<Vec<WeightEntry>> {
        Ok(sqlx::query_as(
            "SELECT id, weight_kg, measured_at
             FROM weight_entries
             WHERE user_id = $1
             ORDER BY measured_at DESC, id DESC
             LIMIT $2",
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?)
    }

    #[tracing::instrument(name = "data::weight_chart_points", level = "info", skip_all)]
    pub async fn weight_chart_points(
        &self,
        user_id: Uuid,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        max_points: i64,
        time_zone: &str,
    ) -> Result<Vec<WeightPoint>> {
        Ok(sqlx::query_as(
            "WITH measurements AS (
                 SELECT measured_at, weight_kg,
                        (measured_at AT TIME ZONE $5)::date AS day
                 FROM weight_entries
                 WHERE user_id = $1
                   AND ($2::timestamptz IS NULL OR measured_at >=
                        (((($2 AT TIME ZONE $5)::date - 6)::timestamp) AT TIME ZONE $5))
                   AND ($3::timestamptz IS NULL OR measured_at <= $3)
             ), days AS (
                 SELECT day, avg(weight_kg) AS weight_kg
                 FROM measurements
                 GROUP BY day
             ), daily_trend AS (
                 SELECT day,
                        avg(weight_kg) OVER (
                            ORDER BY day::timestamp
                            RANGE BETWEEN INTERVAL '6 days' PRECEDING AND CURRENT ROW
                        ) AS trend_weight_kg
                 FROM days
             ), visible_days AS (
                 SELECT day, min(measured_at) AS measured_at,
                        avg(weight_kg) AS weight_kg
                 FROM measurements
                 WHERE $2::timestamptz IS NULL OR measured_at >= $2
                 GROUP BY day
             ), points AS (
                 SELECT visible_days.measured_at, visible_days.weight_kg,
                        daily_trend.trend_weight_kg
                 FROM visible_days
                 JOIN daily_trend USING (day)
             ), bounds AS (
                 SELECT min(measured_at) AS first_at, max(measured_at) AS last_at
                 FROM points
             )
             SELECT to_timestamp(avg(extract(epoch FROM points.measured_at))) AS measured_at,
                    avg(points.weight_kg) AS weight_kg,
                    avg(points.trend_weight_kg) AS trend_weight_kg
             FROM points
             CROSS JOIN bounds
             GROUP BY floor(
                 extract(epoch FROM (points.measured_at - bounds.first_at))
                 / greatest(
                     extract(epoch FROM (bounds.last_at - bounds.first_at))
                     / greatest($4 - 1, 1),
                     1
                 )
             )
             ORDER BY measured_at",
        )
        .bind(user_id)
        .bind(from)
        .bind(to)
        .bind(max_points)
        .bind(time_zone)
        .fetch_all(&self.pool)
        .await?)
    }

    #[tracing::instrument(name = "data::latest_weight_entry", level = "info", skip_all)]
    pub async fn latest_weight_entry(&self, user_id: Uuid) -> Result<Option<WeightEntry>> {
        Ok(sqlx::query_as(
            "SELECT id, weight_kg, measured_at
             FROM weight_entries
             WHERE user_id = $1
             ORDER BY measured_at DESC, id DESC
             LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?)
    }

    #[tracing::instrument(name = "data::first_weight_entry_since", level = "info", skip_all)]
    pub async fn first_weight_entry_since(
        &self,
        user_id: Uuid,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Result<Option<WeightEntry>> {
        Ok(sqlx::query_as(
            "SELECT id, weight_kg, measured_at
             FROM weight_entries
             WHERE user_id = $1
               AND ($2::timestamptz IS NULL OR measured_at >= $2)
               AND ($3::timestamptz IS NULL OR measured_at <= $3)
             ORDER BY measured_at, id
             LIMIT 1",
        )
        .bind(user_id)
        .bind(from)
        .bind(to)
        .fetch_optional(&self.pool)
        .await?)
    }

    #[tracing::instrument(name = "data::last_weight_entry_in_range", level = "info", skip_all)]
    pub async fn last_weight_entry_in_range(
        &self,
        user_id: Uuid,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Result<Option<WeightEntry>> {
        Ok(sqlx::query_as(
            "SELECT id, weight_kg, measured_at
             FROM weight_entries
             WHERE user_id = $1
               AND ($2::timestamptz IS NULL OR measured_at >= $2)
               AND ($3::timestamptz IS NULL OR measured_at <= $3)
             ORDER BY measured_at DESC, id DESC
             LIMIT 1",
        )
        .bind(user_id)
        .bind(from)
        .bind(to)
        .fetch_optional(&self.pool)
        .await?)
    }

    #[tracing::instrument(name = "data::add_weight", level = "info", skip_all)]
    pub async fn add_weight(
        &self,
        user_id: Uuid,
        weight_kg: f64,
        measured_at: DateTime<Utc>,
    ) -> Result<WeightEntry> {
        Ok(sqlx::query_as(
            "INSERT INTO weight_entries (id, user_id, weight_kg, measured_at)
             VALUES ($1, $2, $3, $4)
             RETURNING id, weight_kg, measured_at",
        )
        .bind(Uuid::now_v7())
        .bind(user_id)
        .bind(weight_kg)
        .bind(measured_at)
        .fetch_one(&self.pool)
        .await?)
    }

    #[tracing::instrument(name = "data::update_weight", level = "info", skip_all)]
    pub async fn update_weight(
        &self,
        user_id: Uuid,
        entry_id: Uuid,
        weight_kg: f64,
        measured_at: DateTime<Utc>,
    ) -> Result<Option<WeightEntry>> {
        Ok(sqlx::query_as(
            "UPDATE weight_entries
             SET weight_kg = $3, measured_at = $4, updated_at = now()
             WHERE id = $1 AND user_id = $2
             RETURNING id, weight_kg, measured_at",
        )
        .bind(entry_id)
        .bind(user_id)
        .bind(weight_kg)
        .bind(measured_at)
        .fetch_optional(&self.pool)
        .await?)
    }

    #[tracing::instrument(name = "data::delete_weight", level = "info", skip_all)]
    pub async fn delete_weight(&self, user_id: Uuid, entry_id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM weight_entries WHERE id = $1 AND user_id = $2")
            .bind(entry_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() == 1)
    }
}

#[cfg(test)]
mod tests;
