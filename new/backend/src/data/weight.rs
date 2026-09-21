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

impl Data {
    #[tracing::instrument(name = "data::weight_entries", level = "info", skip_all)]
    pub async fn weight_entries(&self, user_id: Uuid) -> Result<Vec<WeightEntry>> {
        Ok(sqlx::query_as(
            "SELECT id, weight_kg, measured_at
             FROM weight_entries
             WHERE user_id = $1
             ORDER BY measured_at DESC, id DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
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
