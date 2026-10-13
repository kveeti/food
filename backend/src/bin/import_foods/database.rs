use std::collections::{BTreeMap, HashMap};

use anyhow::{Result, bail};
use sqlx::{Connection, PgConnection};
use uuid::Uuid;

use crate::model::{CanonicalUnit, ImportBatch, NutrientRegistry};

#[derive(Clone, Copy, Debug)]
pub enum ImportMode {
    Full { snapshot_end: Option<i64> },
    Delta { start: i64, end: i64 },
}

pub struct ImportWriter<'a> {
    connection: &'a mut PgConnection,
    source: &'static str,
    mode: ImportMode,
    registry: NutrientRegistry,
}

impl<'a> ImportWriter<'a> {
    pub async fn start(
        connection: &'a mut PgConnection,
        source: &'static str,
        mode: ImportMode,
    ) -> Result<Self> {
        sqlx::query("SELECT pg_advisory_lock(hashtextextended($1, 0))")
            .bind(format!("food-import:{source}"))
            .execute(&mut *connection)
            .await?;

        match mode {
            ImportMode::Full {
                snapshot_end: Some(snapshot_end),
            } => validate_full(connection, snapshot_end).await?,
            ImportMode::Delta { start, end } => validate_delta(connection, start, end).await?,
            ImportMode::Full { snapshot_end: None } => {}
        }

        sqlx::raw_sql(
            "CREATE TEMP TABLE import_touched_foods (
                 source_id text PRIMARY KEY
             ) ON COMMIT PRESERVE ROWS;
             CREATE TEMP TABLE import_foods (
                 id uuid NOT NULL,
                 source_id text PRIMARY KEY,
                 display_name text NOT NULL,
                 brand text,
                 basis_unit text NOT NULL,
                 source_data text NOT NULL
             ) ON COMMIT PRESERVE ROWS;
             CREATE TEMP TABLE import_food_names (
                 source_id text NOT NULL,
                 name text NOT NULL,
                 locale text
             ) ON COMMIT PRESERVE ROWS;
             CREATE TEMP TABLE import_food_nutrients (
                 source_id text NOT NULL,
                 nutrient_code text NOT NULL,
                 value double precision NOT NULL,
                 PRIMARY KEY (source_id, nutrient_code)
             ) ON COMMIT PRESERVE ROWS;",
        )
        .execute(&mut *connection)
        .await?;

        let rows = sqlx::query_as::<_, (String, String)>(
            "SELECT code, canonical_unit FROM nutrients WHERE NOT is_archived",
        )
        .fetch_all(&mut *connection)
        .await?;
        let mut units = HashMap::with_capacity(rows.len());
        for (code, unit) in rows {
            let unit = match unit.as_str() {
                "g" => CanonicalUnit::Grams,
                "kJ" => CanonicalUnit::Kilojoules,
                _ => bail!("canonical nutrient {code} has unsupported unit {unit}"),
            };
            units.insert(code, unit);
        }
        if units.is_empty() {
            bail!("canonical nutrient registry is empty");
        }

        Ok(Self {
            connection,
            source,
            mode,
            registry: NutrientRegistry::new(units),
        })
    }

    pub fn registry(&self) -> &NutrientRegistry {
        &self.registry
    }

    pub async fn write_batch(&mut self, batch: ImportBatch) -> Result<()> {
        let batch = deduplicate(batch);
        if batch.touched_source_ids.is_empty() && batch.foods.is_empty() {
            return Ok(());
        }

        let mut transaction = self.connection.begin().await?;
        if !batch.touched_source_ids.is_empty() {
            sqlx::query(
                "INSERT INTO import_touched_foods (source_id)
                 SELECT source_id FROM UNNEST($1::text[]) AS input(source_id)
                 ON CONFLICT DO NOTHING",
            )
            .bind(&batch.touched_source_ids)
            .execute(&mut *transaction)
            .await?;

            sqlx::query("DELETE FROM import_foods WHERE source_id = ANY($1::text[])")
                .bind(&batch.touched_source_ids)
                .execute(&mut *transaction)
                .await?;
            sqlx::query("DELETE FROM import_food_names WHERE source_id = ANY($1::text[])")
                .bind(&batch.touched_source_ids)
                .execute(&mut *transaction)
                .await?;
            sqlx::query("DELETE FROM import_food_nutrients WHERE source_id = ANY($1::text[])")
                .bind(&batch.touched_source_ids)
                .execute(&mut *transaction)
                .await?;
        }

        if !batch.foods.is_empty() {
            let ids = batch
                .foods
                .iter()
                .map(|_| Uuid::now_v7())
                .collect::<Vec<_>>();
            let source_ids = batch
                .foods
                .iter()
                .map(|food| food.source_id.clone())
                .collect::<Vec<_>>();
            let display_names = batch
                .foods
                .iter()
                .map(|food| food.display_name.clone())
                .collect::<Vec<_>>();
            let brands = batch
                .foods
                .iter()
                .map(|food| food.brand.clone())
                .collect::<Vec<_>>();
            let basis_units = batch
                .foods
                .iter()
                .map(|food| food.basis_unit)
                .collect::<Vec<_>>();
            let source_data = batch
                .foods
                .iter()
                .map(|food| food.source_data.to_string())
                .collect::<Vec<_>>();

            sqlx::query(
                "INSERT INTO import_foods
                     (id, source_id, display_name, brand, basis_unit, source_data)
                 SELECT *
                 FROM UNNEST($1::uuid[], $2::text[], $3::text[], $4::text[], $5::text[], $6::text[])
                 ON CONFLICT (source_id) DO UPDATE SET
                     id = EXCLUDED.id,
                     display_name = EXCLUDED.display_name,
                     brand = EXCLUDED.brand,
                     basis_unit = EXCLUDED.basis_unit,
                     source_data = EXCLUDED.source_data",
            )
            .bind(&ids)
            .bind(&source_ids)
            .bind(&display_names)
            .bind(&brands)
            .bind(&basis_units)
            .bind(&source_data)
            .execute(&mut *transaction)
            .await?;

            let mut name_source_ids = Vec::new();
            let mut names = Vec::new();
            let mut locales = Vec::new();
            let mut value_source_ids = Vec::new();
            let mut nutrient_codes = Vec::new();
            let mut values = Vec::new();
            for food in &batch.foods {
                for name in &food.names {
                    name_source_ids.push(food.source_id.clone());
                    names.push(name.name.clone());
                    locales.push(name.locale.clone());
                }
                for (code, value) in &food.nutrients {
                    value_source_ids.push(food.source_id.clone());
                    nutrient_codes.push(code.clone());
                    values.push(*value);
                }
            }

            if !names.is_empty() {
                sqlx::query(
                    "INSERT INTO import_food_names (source_id, name, locale)
                     SELECT * FROM UNNEST($1::text[], $2::text[], $3::text[])",
                )
                .bind(&name_source_ids)
                .bind(&names)
                .bind(&locales)
                .execute(&mut *transaction)
                .await?;
            }
            if !values.is_empty() {
                sqlx::query(
                    "INSERT INTO import_food_nutrients (source_id, nutrient_code, value)
                     SELECT * FROM UNNEST($1::text[], $2::text[], $3::float8[])",
                )
                .bind(&value_source_ids)
                .bind(&nutrient_codes)
                .bind(&values)
                .execute(&mut *transaction)
                .await?;
            }
        }

        transaction.commit().await?;
        Ok(())
    }

    pub async fn finish(self) -> Result<()> {
        let staged_foods: i64 = sqlx::query_scalar("SELECT count(*) FROM import_foods")
            .fetch_one(&mut *self.connection)
            .await?;
        if matches!(self.mode, ImportMode::Full { .. }) && staged_foods == 0 {
            bail!("full import contained no importable foods");
        }

        let unknown_codes: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT staged.nutrient_code
             FROM import_food_nutrients staged
             LEFT JOIN nutrients ON nutrients.code = staged.nutrient_code
             WHERE nutrients.id IS NULL
             ORDER BY staged.nutrient_code",
        )
        .fetch_all(&mut *self.connection)
        .await?;
        if !unknown_codes.is_empty() {
            bail!(
                "staged values refer to unknown canonical nutrients: {}",
                unknown_codes.join(", ")
            );
        }

        let mut transaction = self.connection.begin().await?;
        match self.mode {
            ImportMode::Full { .. } => {
                sqlx::query(
                    "UPDATE foods
                     SET is_archived = true, updated_at = now()
                     WHERE source = $1",
                )
                .bind(self.source)
                .execute(&mut *transaction)
                .await?;
            }
            ImportMode::Delta { .. } => {
                sqlx::query(
                    "UPDATE foods
                     SET is_archived = true, updated_at = now()
                     WHERE source = $1
                       AND source_id IN (SELECT source_id FROM import_touched_foods)",
                )
                .bind(self.source)
                .execute(&mut *transaction)
                .await?;
            }
        }

        sqlx::query(
            "INSERT INTO foods
                 (id, source, source_id, display_name, brand, basis_unit, source_data, is_archived)
             SELECT staged.id, $1, staged.source_id, staged.display_name, staged.brand,
                    staged.basis_unit, staged.source_data::jsonb, false
             FROM import_foods staged
             ON CONFLICT (source, source_id) DO UPDATE SET
                 display_name = EXCLUDED.display_name,
                 brand = EXCLUDED.brand,
                 basis_unit = EXCLUDED.basis_unit,
                 source_data = EXCLUDED.source_data,
                 is_archived = false,
                 updated_at = now()",
        )
        .bind(self.source)
        .execute(&mut *transaction)
        .await?;

        let affected = match self.mode {
            ImportMode::Full { .. } => "foods.source = $1",
            ImportMode::Delta { .. } => {
                "foods.source = $1 AND foods.source_id IN (SELECT source_id FROM import_touched_foods)"
            }
        };
        sqlx::query(&format!(
            "DELETE FROM food_names
             USING foods
             WHERE food_names.food_id = foods.id AND {affected}"
        ))
        .bind(self.source)
        .execute(&mut *transaction)
        .await?;
        sqlx::query(&format!(
            "DELETE FROM food_nutrients
             USING foods
             WHERE food_nutrients.food_id = foods.id AND {affected}"
        ))
        .bind(self.source)
        .execute(&mut *transaction)
        .await?;

        sqlx::query(
            "INSERT INTO food_names (food_id, name, locale)
             SELECT foods.id, staged.name, staged.locale
             FROM import_food_names staged
             JOIN foods ON foods.source = $1 AND foods.source_id = staged.source_id
             ON CONFLICT DO NOTHING",
        )
        .bind(self.source)
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            "INSERT INTO food_nutrients (food_id, nutrient_id, value)
             SELECT foods.id, nutrients.id, staged.value
             FROM import_food_nutrients staged
             JOIN foods ON foods.source = $1 AND foods.source_id = staged.source_id
             JOIN nutrients ON nutrients.code = staged.nutrient_code",
        )
        .bind(self.source)
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            "SELECT refresh_food_search_vector(foods.id)
             FROM foods
             JOIN import_foods staged ON staged.source_id = foods.source_id
             WHERE foods.source = $1",
        )
        .bind(self.source)
        .execute(&mut *transaction)
        .await?;

        match self.mode {
            ImportMode::Full {
                snapshot_end: Some(snapshot_end),
            } => {
                sqlx::query(
                    "UPDATE off_sync_state
                     SET last_delta_end = $1, last_full_import_at = now(), updated_at = now()
                     WHERE singleton",
                )
                .bind(snapshot_end)
                .execute(&mut *transaction)
                .await?;
            }
            ImportMode::Full { snapshot_end: None } => {}
            ImportMode::Delta { start, end } => {
                validate_delta_transaction(&mut transaction, start, end).await?;
                sqlx::query(
                    "UPDATE off_sync_state
                     SET last_delta_end = $1, updated_at = now()
                     WHERE singleton",
                )
                .bind(end)
                .execute(&mut *transaction)
                .await?;
            }
        }

        transaction.commit().await?;
        sqlx::query("SELECT pg_advisory_unlock(hashtextextended($1, 0))")
            .bind(format!("food-import:{}", self.source))
            .execute(&mut *self.connection)
            .await?;
        Ok(())
    }
}

fn deduplicate(batch: ImportBatch) -> ImportBatch {
    let mut touched_source_ids = batch.touched_source_ids;
    touched_source_ids.extend(batch.foods.iter().map(|food| food.source_id.clone()));
    touched_source_ids.sort_unstable();
    touched_source_ids.dedup();

    let mut foods = BTreeMap::new();
    for food in batch.foods {
        foods.insert(food.source_id.clone(), food);
    }
    ImportBatch {
        touched_source_ids,
        foods: foods.into_values().collect(),
    }
}

async fn validate_full(connection: &mut PgConnection, snapshot_end: i64) -> Result<()> {
    if snapshot_end <= 0 {
        bail!("full snapshot timestamp must be positive");
    }
    let cursor: Option<i64> =
        sqlx::query_scalar("SELECT last_delta_end FROM off_sync_state WHERE singleton")
            .fetch_one(connection)
            .await?;
    if let Some(cursor) = cursor
        && snapshot_end < cursor
    {
        bail!("full snapshot at {snapshot_end} is older than OFF cursor {cursor}");
    }
    Ok(())
}

async fn validate_delta(connection: &mut PgConnection, start: i64, end: i64) -> Result<()> {
    if start >= end {
        bail!("delta start must be before its end");
    }
    let cursor: Option<i64> =
        sqlx::query_scalar("SELECT last_delta_end FROM off_sync_state WHERE singleton")
            .fetch_one(connection)
            .await?;
    validate_cursor(cursor, start, end)
}

async fn validate_delta_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    start: i64,
    end: i64,
) -> Result<()> {
    let cursor: Option<i64> =
        sqlx::query_scalar("SELECT last_delta_end FROM off_sync_state WHERE singleton FOR UPDATE")
            .fetch_one(&mut **transaction)
            .await?;
    validate_cursor(cursor, start, end)
}

fn validate_cursor(cursor: Option<i64>, start: i64, end: i64) -> Result<()> {
    let Some(cursor) = cursor else {
        bail!("OFF delta needs a completed full import");
    };
    if end <= cursor {
        bail!("OFF delta ending at {end} was already applied; cursor is {cursor}");
    }
    if start > cursor {
        bail!("OFF delta gap: cursor is {cursor}, next delta starts at {start}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delta_cursor_accepts_overlap_and_rejects_gaps() {
        assert!(validate_cursor(Some(100), 90, 110).is_ok());
        assert!(validate_cursor(Some(100), 101, 110).is_err());
        assert!(validate_cursor(Some(100), 90, 100).is_err());
        assert!(validate_cursor(None, 90, 110).is_err());
    }
}
