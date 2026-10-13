use anyhow::Result;
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;

use super::Data;

#[cfg(test)]
mod tests;

#[derive(sqlx::FromRow, Serialize)]
pub struct Food {
    pub id: String,
    pub display_name: String,
    pub brand: Option<String>,
    pub basis_unit: String,
    pub source: Option<String>,
}

#[derive(Serialize)]
pub struct FoodDetail {
    #[serde(flatten)]
    pub food: Food,
    pub nutrients: serde_json::Value,
}

#[derive(sqlx::FromRow, Serialize)]
pub struct FoodEntry {
    pub id: String,
    pub meal_id: Option<String>,
    pub meal_name: Option<String>,
    pub food_id: Option<String>,
    pub food_name: String,
    pub food_brand: Option<String>,
    pub amount: f64,
    pub unit: String,
    pub eaten_at: DateTime<Utc>,
    pub nutrients: serde_json::Value,
}

#[derive(Serialize)]
pub struct FoodMeal {
    pub id: Option<String>,
    pub name: Option<String>,
    pub started_at: DateTime<Utc>,
    pub entries: Vec<FoodEntry>,
}

#[derive(sqlx::FromRow)]
struct MealEntryRow {
    #[sqlx(flatten)]
    entry: FoodEntry,
    meal_started_at: DateTime<Utc>,
}

impl Data {
    #[tracing::instrument(name = "data::meal_suggestion", level = "info", skip_all)]
    pub async fn meal_suggestion(
        &self,
        user: Uuid,
        day: NaiveDate,
        timezone: &str,
    ) -> Result<MealSuggestion> {
        let row = sqlx::query_as::<_, (Option<String>, Option<String>, Option<String>)>(
            "WITH target AS (
                SELECT (($2::date + (now() AT TIME ZONE $3)::time)::timestamp AT TIME ZONE $3) AS at
             ), previous AS (
                SELECT m.id, m.name, e.eaten_at
                FROM food_entries e JOIN meals m ON m.id = e.meal_id AND m.user_id = e.user_id
                WHERE e.user_id = $1
                  AND (e.eaten_at AT TIME ZONE $3)::date = $2
                ORDER BY e.eaten_at DESC, e.id DESC LIMIT 1
             ), history AS (
                SELECT m.name, (m.started_at AT TIME ZONE $3)::date AS day,
                    least(abs(extract(epoch FROM ((m.started_at AT TIME ZONE $3)::time -
                        (now() AT TIME ZONE $3)::time))),
                        86400 - abs(extract(epoch FROM ((m.started_at AT TIME ZONE $3)::time -
                        (now() AT TIME ZONE $3)::time)))) AS distance
                FROM meals m
                WHERE m.user_id = $1 AND m.name IN ('Breakfast', 'Lunch', 'Snack', 'Dinner')
                  AND (m.started_at AT TIME ZONE $3)::date >= $2::date - 28
                  AND (m.started_at AT TIME ZONE $3)::date < $2
                  AND EXISTS (SELECT 1 FROM food_entries WHERE meal_id = m.id AND user_id = $1)
             )
             SELECT (SELECT id::text FROM previous), (SELECT name FROM previous),
                CASE WHEN EXISTS (SELECT 1 FROM previous, target
                    WHERE target.at - previous.eaten_at BETWEEN interval '0 seconds' AND interval '2 hours')
                THEN 'continue_previous'
                ELSE (SELECT lower(name) FROM history
                    WHERE distance <= 5400 AND
                        (extract(isodow FROM day) = extract(isodow FROM $2::date) OR day >= $2::date - 7)
                    ORDER BY (extract(isodow FROM day) = extract(isodow FROM $2::date)) DESC,
                        distance, day DESC, name LIMIT 1)
                END",
        )
        .bind(user).bind(day).bind(timezone).fetch_one(&self.pool).await?;
        Ok(MealSuggestion {
            previous_meal_id: row.0,
            previous_meal_name: row.1,
            meal: row.2,
        })
    }

    #[tracing::instrument(name = "data::search_foods", level = "info", skip_all)]
    pub async fn search_foods(&self, user: Uuid, query: &str) -> Result<Vec<Food>> {
        Ok(sqlx::query_as(
            "WITH q AS (
                SELECT to_tsquery('simple', coalesce((
                    SELECT string_agg(quote_literal(word) || ':*', ' & ')
                    FROM unnest(tsvector_to_array(to_tsvector('simple', $2))) word
                ), '')) AS prefix,
                plainto_tsquery('finnish', $2) AS fi,
                plainto_tsquery('swedish', $2) AS sv,
                plainto_tsquery('english', $2) AS en
             )
             SELECT f.id::text, display_name, brand, basis_unit, source
             FROM foods f CROSS JOIN q
             WHERE NOT is_archived AND (owner_user_id IS NULL OR owner_user_id = $1)
               AND (search_vector @@ q.prefix OR search_fi_vector @@ q.fi
                 OR search_sv_vector @@ q.sv OR search_en_vector @@ q.en)
             ORDER BY (lower(display_name) = lower($2)) DESC,
                ts_rank(search_vector, q.prefix) DESC, display_name, f.id
             LIMIT 30",
        )
        .bind(user)
        .bind(query)
        .fetch_all(&self.pool)
        .await?)
    }

    #[tracing::instrument(name = "data::food", level = "info", skip_all)]
    pub async fn food(&self, user: Uuid, id: Uuid) -> Result<Option<FoodDetail>> {
        let mut tx = self.pool.begin().await?;
        let food = sqlx::query_as::<_, Food>(
            "SELECT id::text, display_name, brand, basis_unit, source FROM foods
             WHERE id = $1 AND NOT is_archived AND (owner_user_id IS NULL OR owner_user_id = $2)
             FOR SHARE",
        )
        .bind(id)
        .bind(user)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(food) = food else { return Ok(None) };
        let nutrients = sqlx::query_scalar(
            "SELECT coalesce(jsonb_agg(jsonb_build_object(
                'code', n.code, 'name', n.display_name, 'unit', n.display_unit,
                'value', fn.value * n.display_scale, 'show_by_default', n.show_by_default
             ) ORDER BY n.display_order), '[]'::jsonb)
             FROM food_nutrients fn JOIN nutrients n ON n.id = fn.nutrient_id
             WHERE fn.food_id = $1",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(Some(FoodDetail { food, nutrients }))
    }

    #[tracing::instrument(name = "data::food_meals", level = "info", skip_all)]
    pub async fn food_meals(
        &self,
        user: Uuid,
        date: NaiveDate,
        timezone: &str,
    ) -> Result<Vec<FoodMeal>> {
        let rows = sqlx::query_as::<_, MealEntryRow>(&format!(
            "SELECT entry.*, coalesce(m.started_at, entry.eaten_at) AS meal_started_at
             FROM ({ENTRY_SELECT} WHERE e.user_id = $1
             AND eaten_at >= ($2::date::timestamp AT TIME ZONE $3)
             AND eaten_at < (($2::date + 1)::timestamp AT TIME ZONE $3)) entry
             LEFT JOIN meals m ON m.id = entry.meal_id::uuid AND m.user_id = $1
             ORDER BY meal_started_at DESC, coalesce(entry.meal_id, entry.id) DESC,
                 entry.eaten_at DESC, entry.id DESC"
        ))
        .bind(user)
        .bind(date)
        .bind(timezone)
        .fetch_all(&self.pool)
        .await?;
        let mut meals: Vec<FoodMeal> = Vec::new();
        for row in rows {
            if row.entry.meal_id.is_none()
                || meals.last().is_none_or(|meal| meal.id != row.entry.meal_id)
            {
                meals.push(FoodMeal {
                    id: row.entry.meal_id.clone(),
                    name: row.entry.meal_name.clone(),
                    started_at: row.meal_started_at,
                    entries: Vec::new(),
                });
            }
            meals
                .last_mut()
                .expect("meal was just added")
                .entries
                .push(row.entry);
        }
        Ok(meals)
    }

    #[tracing::instrument(name = "data::add_food", level = "info", skip_all)]
    pub async fn add_food(&self, user: Uuid, input: NewFoodEntry<'_>) -> Result<Option<FoodEntry>> {
        let mut tx = self.pool.begin().await?;
        let entry_id = Uuid::now_v7();
        // Serialize meal selection for this user when foods are logged together.
        sqlx::query("SELECT id FROM users WHERE id = $1 FOR UPDATE")
            .bind(user)
            .execute(&mut *tx)
            .await?;
        // Lock the definition while copying identity and nutrients from the catalog.
        let available = sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM foods WHERE id = $1 AND NOT is_archived
             AND (owner_user_id IS NULL OR owner_user_id = $2) AND basis_unit = $3 FOR SHARE",
        )
        .bind(input.food_id)
        .bind(user)
        .bind(input.unit)
        .fetch_optional(&mut *tx)
        .await?;
        if available.is_none() {
            return Ok(None);
        }
        let meal_id = if let Some(name) = input.meal.name() {
            sqlx::query_scalar::<_, Uuid>(
                "INSERT INTO meals (id, user_id, name, started_at)
                 VALUES ($1, $2, $3,
                    (($4::date + (now() AT TIME ZONE $5)::time)::timestamp AT TIME ZONE $5))
                 RETURNING id",
            )
            .bind(Uuid::now_v7())
            .bind(user)
            .bind(name)
            .bind(input.date)
            .bind(input.timezone)
            .fetch_one(&mut *tx)
            .await?
        } else {
            let previous = sqlx::query_scalar::<_, Uuid>(
                "SELECT m.id FROM meals m
                 JOIN food_entries e ON e.meal_id = m.id AND e.user_id = m.user_id
                 WHERE m.user_id = $1
                   AND e.eaten_at >= ($2::date::timestamp AT TIME ZONE $3)
                   AND e.eaten_at < (($2::date + 1)::timestamp AT TIME ZONE $3)
                 ORDER BY e.eaten_at DESC, e.id DESC LIMIT 1",
            )
            .bind(user)
            .bind(input.date)
            .bind(input.timezone)
            .fetch_optional(&mut *tx)
            .await?;
            let Some(previous) = previous else {
                return Ok(None);
            };
            if Some(previous) != input.meal_id {
                return Ok(None);
            }
            previous
        };
        sqlx::query(
            "INSERT INTO food_entries (id, user_id, food_id, meal_id, amount, unit, eaten_at,
                food_name, food_brand, food_source, food_source_id)
             SELECT $1, $2, id, $7, $3, basis_unit,
                (($4::date + (now() AT TIME ZONE $5)::time)::timestamp AT TIME ZONE $5),
                display_name, brand, source, source_id FROM foods WHERE id = $6",
        )
        .bind(entry_id)
        .bind(user)
        .bind(input.amount)
        .bind(input.date)
        .bind(input.timezone)
        .bind(input.food_id)
        .bind(meal_id)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO food_entry_nutrients (food_entry_id, nutrient_id, basis_value)
             SELECT $1, nutrient_id, value FROM food_nutrients WHERE food_id = $2",
        )
        .bind(entry_id)
        .bind(input.food_id)
        .execute(&mut *tx)
        .await?;
        let entry = sqlx::query_as(&format!("{ENTRY_SELECT} WHERE e.id = $1"))
            .bind(entry_id)
            .fetch_one(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(Some(entry))
    }

    #[tracing::instrument(name = "data::copy_meal", level = "info", skip_all)]
    pub async fn copy_meal(
        &self,
        user: Uuid,
        source: Uuid,
        input: CopyMealInput,
        timezone: &str,
    ) -> Result<Option<Uuid>> {
        if input.entries.is_empty()
            || input.entries.iter().any(|entry| {
                !entry.amount.is_finite() || entry.amount <= 0.0 || entry.amount > 100_000.0
            })
        {
            return Ok(None);
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT id FROM users WHERE id = $1 FOR UPDATE")
            .bind(user)
            .execute(&mut *tx)
            .await?;
        let source_ids: Vec<_> = input.entries.iter().map(|entry| entry.entry_id).collect();
        let available = sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM food_entries
             WHERE user_id = $1 AND meal_id = $2 AND id = ANY($3) FOR SHARE",
        )
        .bind(user)
        .bind(source)
        .bind(&source_ids)
        .fetch_all(&mut *tx)
        .await?;
        if available.len() != source_ids.len() {
            return Ok(None);
        }
        let (meal_id, eaten_at) = match input.target {
            CopyMealTarget::New { meal, time } => {
                let Some(name) = meal.name() else {
                    return Ok(None);
                };
                sqlx::query_as::<_, (Uuid, DateTime<Utc>)>(
                    "INSERT INTO meals (id, user_id, name, started_at)
                     VALUES ($1, $2, $3, (($4::date + $5::time) AT TIME ZONE $6))
                     RETURNING id, started_at",
                )
                .bind(Uuid::now_v7())
                .bind(user)
                .bind(name)
                .bind(input.date)
                .bind(time)
                .bind(timezone)
                .fetch_one(&mut *tx)
                .await?
            }
            CopyMealTarget::Existing { meal_id } => {
                let target = sqlx::query_as::<_, (Uuid, DateTime<Utc>)>(
                    "SELECT id, started_at FROM meals
                     WHERE id = $1 AND user_id = $2
                       AND (started_at AT TIME ZONE $4)::date = $3
                       AND EXISTS (SELECT 1 FROM food_entries WHERE meal_id = $1)
                     FOR SHARE",
                )
                .bind(meal_id)
                .bind(user)
                .bind(input.date)
                .bind(timezone)
                .fetch_optional(&mut *tx)
                .await?;
                let Some(target) = target else {
                    return Ok(None);
                };
                target
            }
        };
        let mut new_ids: Vec<_> = input.entries.iter().map(|_| Uuid::now_v7()).collect();
        new_ids.reverse();
        let amounts: Vec<_> = input.entries.iter().map(|entry| entry.amount).collect();
        sqlx::query(
            "INSERT INTO food_entries (id, user_id, food_id, meal_id, amount, unit, eaten_at,
                food_name, food_brand, food_source, food_source_id)
             SELECT copy.id, $1, e.food_id, $2, copy.amount, e.unit, $3,
                e.food_name, e.food_brand, e.food_source, e.food_source_id
             FROM unnest($4::uuid[], $5::uuid[], $6::float8[]) AS copy(source_id, id, amount)
             JOIN food_entries e ON e.id = copy.source_id AND e.user_id = $1",
        )
        .bind(user)
        .bind(meal_id)
        .bind(eaten_at)
        .bind(&source_ids)
        .bind(&new_ids)
        .bind(&amounts)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO food_entry_nutrients (food_entry_id, nutrient_id, basis_value)
             SELECT copy.id, n.nutrient_id, n.basis_value
             FROM unnest($1::uuid[], $2::uuid[]) AS copy(source_id, id)
             JOIN food_entry_nutrients n ON n.food_entry_id = copy.source_id",
        )
        .bind(&source_ids)
        .bind(&new_ids)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(Some(meal_id))
    }

    #[tracing::instrument(name = "data::delete_meal", level = "info", skip_all)]
    pub async fn delete_meal(&self, user: Uuid, id: Uuid) -> Result<bool> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT id FROM users WHERE id = $1 FOR UPDATE")
            .bind(user)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM food_entries WHERE meal_id = $1 AND user_id = $2")
            .bind(id)
            .bind(user)
            .execute(&mut *tx)
            .await?;
        let deleted = sqlx::query("DELETE FROM meals WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user)
            .execute(&mut *tx)
            .await?
            .rows_affected();
        tx.commit().await?;
        Ok(deleted > 0)
    }

    #[tracing::instrument(name = "data::delete_food_entry", level = "info", skip_all)]
    pub async fn delete_food_entry(&self, user: Uuid, id: Uuid) -> Result<bool> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT id FROM users WHERE id = $1 FOR UPDATE")
            .bind(user)
            .execute(&mut *tx)
            .await?;
        let meal = sqlx::query_scalar::<_, Option<Uuid>>(
            "DELETE FROM food_entries WHERE id = $1 AND user_id = $2 RETURNING meal_id",
        )
        .bind(id)
        .bind(user)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(Some(meal_id)) = meal {
            sqlx::query(
                "DELETE FROM meals WHERE id = $1 AND user_id = $2
                AND NOT EXISTS (SELECT 1 FROM food_entries WHERE meal_id = $1)",
            )
            .bind(meal_id)
            .bind(user)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(meal.is_some())
    }

    #[tracing::instrument(name = "data::update_food_entry", level = "info", skip_all)]
    pub async fn update_food_entry(
        &self,
        user: Uuid,
        id: Uuid,
        amount: f64,
    ) -> Result<Option<FoodEntry>> {
        let mut tx = self.pool.begin().await?;
        let changed =
            sqlx::query("UPDATE food_entries SET amount = $1 WHERE id = $2 AND user_id = $3")
                .bind(amount)
                .bind(id)
                .bind(user)
                .execute(&mut *tx)
                .await?
                .rows_affected();
        if changed == 0 {
            return Ok(None);
        }
        let entry = sqlx::query_as(&format!(
            "{ENTRY_SELECT} WHERE e.id = $1 AND e.user_id = $2"
        ))
        .bind(id)
        .bind(user)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(Some(entry))
    }
}

pub struct NewFoodEntry<'a> {
    pub meal: MealChoice,
    pub meal_id: Option<Uuid>,
    pub food_id: Uuid,
    pub amount: f64,
    pub unit: &'a str,
    pub date: NaiveDate,
    pub timezone: &'a str,
}

#[derive(Deserialize)]
pub struct CopyMealInput {
    pub date: NaiveDate,
    pub target: CopyMealTarget,
    pub entries: Vec<CopyMealEntry>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CopyMealTarget {
    New { meal: MealChoice, time: NaiveTime },
    Existing { meal_id: Uuid },
}

#[derive(Deserialize)]
pub struct CopyMealEntry {
    pub entry_id: Uuid,
    pub amount: f64,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MealChoice {
    ContinuePrevious,
    Breakfast,
    Lunch,
    Snack,
    Dinner,
}

impl MealChoice {
    fn name(self) -> Option<&'static str> {
        match self {
            Self::ContinuePrevious => None,
            Self::Breakfast => Some("Breakfast"),
            Self::Lunch => Some("Lunch"),
            Self::Snack => Some("Snack"),
            Self::Dinner => Some("Dinner"),
        }
    }
}

#[derive(Serialize)]
pub struct MealSuggestion {
    pub previous_meal_id: Option<String>,
    pub previous_meal_name: Option<String>,
    pub meal: Option<String>,
}

const ENTRY_SELECT: &str = "SELECT e.id::text, e.food_id::text, e.meal_id::text,
    (SELECT name FROM meals WHERE id = e.meal_id AND user_id = e.user_id) AS meal_name,
    food_name, food_brand,
    amount, unit, eaten_at, coalesce((
        SELECT jsonb_agg(jsonb_build_object(
            'code', n.code, 'name', n.display_name, 'unit', n.display_unit,
            'value', fn.basis_value * n.display_scale * e.amount / 100,
            'show_by_default', n.show_by_default
        ) ORDER BY n.display_order)
        FROM food_entry_nutrients fn JOIN nutrients n ON n.id = fn.nutrient_id
        WHERE fn.food_entry_id = e.id
    ), '[]'::jsonb) AS nutrients FROM food_entries e";
