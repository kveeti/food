use anyhow::Result;
use chrono::NaiveDate;
use sqlx::types::Uuid;

use super::Data;

#[cfg(test)]
mod tests;

#[derive(sqlx::FromRow)]
pub struct NutrientDefinition {
    pub id: i16,
    pub code: String,
    pub name: String,
    pub unit: String,
    pub display_scale: f64,
    pub show_by_default: bool,
}

#[derive(sqlx::FromRow)]
pub struct GoalProfile {
    pub starts_on: NaiveDate,
    pub daily_burn_kj: Option<f64>,
    pub food_adjustment_kj: Option<f64>,
    pub water_ml: Option<i32>,
}

#[derive(sqlx::FromRow)]
pub struct NutrientGoal {
    pub code: String,
    pub value: f64,
}

#[derive(sqlx::FromRow)]
pub struct NutrientTotal {
    pub code: String,
    pub value: f64,
    pub known_entries: i64,
    pub total_entries: i64,
}

pub struct NewGoalProfile<'a> {
    pub user_id: Uuid,
    pub starts_on: NaiveDate,
    pub daily_burn_kj: Option<f64>,
    pub food_adjustment_kj: Option<f64>,
    pub water_ml: Option<i32>,
    pub nutrients: &'a [NewNutrientGoal],
}

pub struct NewNutrientGoal {
    pub nutrient_id: i16,
    pub value: f64,
}

impl Data {
    #[tracing::instrument(name = "data::nutrient_definitions", level = "info", skip_all)]
    pub async fn nutrient_definitions(&self) -> Result<Vec<NutrientDefinition>> {
        Ok(sqlx::query_as(
            "SELECT id, code, display_name AS name, display_unit AS unit,
                display_scale, show_by_default
             FROM nutrients
             WHERE NOT is_archived AND category <> 'energy' AND code <> 'water'
             ORDER BY display_order",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    #[tracing::instrument(name = "data::visible_nutrient_definitions", level = "info", skip_all)]
    pub async fn visible_nutrient_definitions(
        &self,
        user_id: Uuid,
        date: NaiveDate,
    ) -> Result<Vec<NutrientDefinition>> {
        Ok(sqlx::query_as(
            "WITH profile AS (
                SELECT id
                FROM goal_profiles
                WHERE user_id = $1 AND starts_on <= $2
                ORDER BY starts_on DESC
                LIMIT 1
             )
             SELECT n.id, n.code, n.display_name AS name, n.display_unit AS unit,
                n.display_scale, n.show_by_default
             FROM nutrients n
             WHERE NOT n.is_archived AND n.category <> 'energy' AND n.code <> 'water'
               AND (n.show_by_default OR EXISTS (
                    SELECT 1
                    FROM nutrient_goals ng JOIN profile p ON p.id = ng.goal_profile_id
                    WHERE ng.nutrient_id = n.id
               ))
             ORDER BY n.display_order",
        )
        .bind(user_id)
        .bind(date)
        .fetch_all(&self.pool)
        .await?)
    }

    #[tracing::instrument(name = "data::goal_profile", level = "info", skip_all)]
    pub async fn goal_profile(
        &self,
        user_id: Uuid,
        date: NaiveDate,
    ) -> Result<Option<(GoalProfile, Vec<NutrientGoal>)>> {
        let Some(profile) = sqlx::query_as::<_, GoalProfile>(
            "SELECT starts_on, daily_burn_kj, food_adjustment_kj, water_ml
             FROM goal_profiles
             WHERE user_id = $1 AND starts_on <= $2
             ORDER BY starts_on DESC
             LIMIT 1",
        )
        .bind(user_id)
        .bind(date)
        .fetch_optional(&self.pool)
        .await?
        else {
            return Ok(None);
        };

        let goals = sqlx::query_as::<_, NutrientGoal>(
            "SELECT n.code, ng.value * n.display_scale AS value
             FROM goal_profiles gp
             JOIN nutrient_goals ng ON ng.goal_profile_id = gp.id
             JOIN nutrients n ON n.id = ng.nutrient_id
             WHERE gp.user_id = $1 AND gp.starts_on = $2
             ORDER BY n.display_order",
        )
        .bind(user_id)
        .bind(profile.starts_on)
        .fetch_all(&self.pool)
        .await?;

        Ok(Some((profile, goals)))
    }

    #[tracing::instrument(name = "data::visible_nutrient_totals", level = "info", skip_all)]
    pub async fn visible_nutrient_totals(
        &self,
        user_id: Uuid,
        date: NaiveDate,
        timezone: &str,
    ) -> Result<Vec<NutrientTotal>> {
        Ok(sqlx::query_as(
            "WITH profile AS MATERIALIZED (
                SELECT id
                FROM goal_profiles
                WHERE user_id = $1 AND starts_on <= $2
                ORDER BY starts_on DESC
                LIMIT 1
             ), entries AS MATERIALIZED (
                SELECT id, amount
                FROM food_entries
                WHERE user_id = $1
                  AND eaten_at >= ($2::date::timestamp AT TIME ZONE $3)
                  AND eaten_at < (($2::date + 1)::timestamp AT TIME ZONE $3)
             ), totals AS (
                SELECT n.id AS nutrient_id,
                    sum(fn.basis_value * n.display_scale * e.amount / 100) AS value,
                    count(*) AS known_entries
                FROM entries e
                JOIN food_entry_nutrients fn ON fn.food_entry_id = e.id
                JOIN nutrients n ON n.id = fn.nutrient_id
                GROUP BY n.id
             )
             SELECT n.code, coalesce(t.value, 0)::double precision AS value,
                coalesce(t.known_entries, 0)::bigint AS known_entries,
                (SELECT count(*) FROM entries)::bigint AS total_entries
             FROM nutrients n
             LEFT JOIN totals t ON t.nutrient_id = n.id
             WHERE NOT n.is_archived
               AND (n.code = 'energy' OR (
                    n.category <> 'energy' AND n.code <> 'water'
                    AND (n.show_by_default OR EXISTS (
                        SELECT 1
                        FROM nutrient_goals ng JOIN profile p ON p.id = ng.goal_profile_id
                        WHERE ng.nutrient_id = n.id
                    ))
               ))
             ORDER BY n.display_order",
        )
        .bind(user_id)
        .bind(date)
        .bind(timezone)
        .fetch_all(&self.pool)
        .await?)
    }

    #[tracing::instrument(name = "data::save_goal_profile", level = "info", skip_all)]
    pub async fn save_goal_profile(&self, profile: NewGoalProfile<'_>) -> Result<()> {
        let mut transaction = self.pool.begin().await?;
        let profile_id = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO goal_profiles (
                id, user_id, starts_on, daily_burn_kj, food_adjustment_kj, water_ml
             ) VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (user_id, starts_on) DO UPDATE SET
                daily_burn_kj = EXCLUDED.daily_burn_kj,
                food_adjustment_kj = EXCLUDED.food_adjustment_kj,
                water_ml = EXCLUDED.water_ml,
                updated_at = now()
             RETURNING id",
        )
        .bind(Uuid::now_v7())
        .bind(profile.user_id)
        .bind(profile.starts_on)
        .bind(profile.daily_burn_kj)
        .bind(profile.food_adjustment_kj)
        .bind(profile.water_ml)
        .fetch_one(&mut *transaction)
        .await?;

        sqlx::query("DELETE FROM nutrient_goals WHERE goal_profile_id = $1")
            .bind(profile_id)
            .execute(&mut *transaction)
            .await?;
        let nutrient_ids: Vec<_> = profile
            .nutrients
            .iter()
            .map(|goal| goal.nutrient_id)
            .collect();
        let nutrient_values: Vec<_> = profile.nutrients.iter().map(|goal| goal.value).collect();
        sqlx::query(
            "INSERT INTO nutrient_goals (goal_profile_id, nutrient_id, value)
             SELECT $1, goals.nutrient_id, goals.value
             FROM unnest($2::smallint[], $3::double precision[])
                AS goals(nutrient_id, value)",
        )
        .bind(profile_id)
        .bind(nutrient_ids)
        .bind(nutrient_values)
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;
        Ok(())
    }
}
