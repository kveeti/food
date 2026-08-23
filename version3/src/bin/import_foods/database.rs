use std::error::Error;

use sqlx::{Connection, PgConnection};

use super::model::FoodImport;

pub async fn write_import(
    connection: &mut PgConnection,
    import: FoodImport,
) -> Result<(), Box<dyn Error>> {
    let mut transaction = connection.begin().await?;
    let source = import.source;
    sqlx::raw_sql(
        "ALTER TABLE foods DISABLE TRIGGER foods_search_vector_trigger;
         ALTER TABLE food_aliases DISABLE TRIGGER food_aliases_search_vector_trigger;",
    )
    .execute(&mut *transaction)
    .await?;

    let definitions = import.nutrients.values().collect::<Vec<_>>();
    let codes = definitions
        .iter()
        .map(|nutrient| nutrient.code.clone())
        .collect::<Vec<_>>();
    let names = definitions
        .iter()
        .map(|nutrient| nutrient.display_name.clone())
        .collect::<Vec<_>>();
    let units = definitions
        .iter()
        .map(|nutrient| nutrient.unit.clone())
        .collect::<Vec<_>>();
    let categories = definitions
        .iter()
        .map(|nutrient| nutrient.category.clone())
        .collect::<Vec<_>>();
    let orders = definitions
        .iter()
        .map(|nutrient| nutrient.display_order)
        .collect::<Vec<_>>();
    sqlx::query(
        "INSERT INTO nutrients (code, display_name, unit, category, display_order)
         SELECT * FROM UNNEST($1::text[], $2::text[], $3::text[], $4::text[], $5::int4[])
         ON CONFLICT (code) DO UPDATE SET
             display_name = EXCLUDED.display_name,
             unit = EXCLUDED.unit,
             category = EXCLUDED.category,
             display_order = EXCLUDED.display_order,
             updated_at = now()",
    )
    .bind(&codes)
    .bind(&names)
    .bind(&units)
    .bind(&categories)
    .bind(&orders)
    .execute(&mut *transaction)
    .await?;

    let mappings = import.mappings.values().collect::<Vec<_>>();
    let mapping_keys = mappings
        .iter()
        .map(|mapping| mapping.source_key.clone())
        .collect::<Vec<_>>();
    let mapping_names = mappings
        .iter()
        .map(|mapping| mapping.source_name.clone())
        .collect::<Vec<_>>();
    let mapping_codes = mappings
        .iter()
        .map(|mapping| mapping.nutrient_code.clone())
        .collect::<Vec<_>>();
    let mapping_units = mappings
        .iter()
        .map(|mapping| mapping.source_unit.clone())
        .collect::<Vec<_>>();
    sqlx::query(
        "INSERT INTO nutrient_source_keys (
             source, source_key, source_name, nutrient_id, source_unit
         )
         SELECT $1, input.source_key, input.source_name, nutrients.id, input.source_unit
         FROM UNNEST($2::text[], $3::text[], $4::text[], $5::text[])
              AS input(source_key, source_name, nutrient_code, source_unit)
         JOIN nutrients ON nutrients.code = input.nutrient_code
         ON CONFLICT (source, source_key) DO UPDATE SET
             source_name = EXCLUDED.source_name,
             nutrient_id = EXCLUDED.nutrient_id,
             source_unit = EXCLUDED.source_unit",
    )
    .bind(source)
    .bind(&mapping_keys)
    .bind(&mapping_names)
    .bind(&mapping_codes)
    .bind(&mapping_units)
    .execute(&mut *transaction)
    .await?;

    sqlx::query("UPDATE foods SET is_archived = true, updated_at = now() WHERE source = $1")
        .bind(source)
        .execute(&mut *transaction)
        .await?;

    let source_ids = import
        .foods
        .iter()
        .map(|food| food.source_id.clone())
        .collect::<Vec<_>>();
    let display_names = import
        .foods
        .iter()
        .map(|food| food.display_name.clone())
        .collect::<Vec<_>>();
    let brands = import
        .foods
        .iter()
        .map(|food| food.brand.clone())
        .collect::<Vec<_>>();
    let source_data = import
        .foods
        .iter()
        .map(|food| food.source_data.to_string())
        .collect::<Vec<_>>();
    sqlx::query(
        "INSERT INTO foods (
             source, source_id, display_name, brand, basis_unit, source_data, is_archived
         )
         SELECT $1, input.source_id, input.display_name, NULLIF(input.brand, ''), 'g',
                input.source_data::jsonb, false
         FROM UNNEST($2::text[], $3::text[], $4::text[], $5::text[])
              AS input(source_id, display_name, brand, source_data)
         ON CONFLICT (source, source_id) WHERE source_id IS NOT NULL DO UPDATE SET
             display_name = EXCLUDED.display_name,
             brand = EXCLUDED.brand,
             basis_unit = EXCLUDED.basis_unit,
             source_data = EXCLUDED.source_data,
             is_archived = false,
             updated_at = now()",
    )
    .bind(source)
    .bind(&source_ids)
    .bind(&display_names)
    .bind(&brands)
    .bind(&source_data)
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        "DELETE FROM food_aliases
         USING foods
         WHERE food_aliases.food_id = foods.id AND foods.source = $1",
    )
    .bind(source)
    .execute(&mut *transaction)
    .await?;

    let mut alias_source_ids = Vec::new();
    let mut alias_names = Vec::new();
    let mut alias_locales = Vec::new();
    for food in &import.foods {
        for alias in &food.aliases {
            alias_source_ids.push(food.source_id.clone());
            alias_names.push(alias.name.clone());
            alias_locales.push(alias.locale.clone());
        }
    }
    sqlx::query(
        "INSERT INTO food_aliases (food_id, name, locale)
         SELECT foods.id, input.name, NULLIF(input.locale, '')
         FROM UNNEST($2::text[], $3::text[], $4::text[])
              AS input(source_id, name, locale)
         JOIN foods ON foods.source = $1 AND foods.source_id = input.source_id
         ON CONFLICT DO NOTHING",
    )
    .bind(source)
    .bind(&alias_source_ids)
    .bind(&alias_names)
    .bind(&alias_locales)
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        "DELETE FROM food_nutrients
         USING foods
         WHERE food_nutrients.food_id = foods.id AND foods.source = $1",
    )
    .bind(source)
    .execute(&mut *transaction)
    .await?;

    let mut value_source_ids = Vec::new();
    let mut value_codes = Vec::new();
    let mut values = Vec::new();
    for food in &import.foods {
        for (code, value) in &food.nutrients {
            value_source_ids.push(food.source_id.clone());
            value_codes.push(code.clone());
            values.push(*value);
        }
    }
    sqlx::query(
        "INSERT INTO food_nutrients (food_id, nutrient_id, value)
         SELECT foods.id, nutrients.id, input.value
         FROM UNNEST($2::text[], $3::text[], $4::float8[])
              AS input(source_id, nutrient_code, value)
         JOIN foods ON foods.source = $1 AND foods.source_id = input.source_id
         JOIN nutrients ON nutrients.code = input.nutrient_code
         ON CONFLICT (food_id, nutrient_id) DO UPDATE SET value = EXCLUDED.value",
    )
    .bind(source)
    .bind(&value_source_ids)
    .bind(&value_codes)
    .bind(&values)
    .execute(&mut *transaction)
    .await?;

    sqlx::raw_sql(
        "ALTER TABLE foods ENABLE TRIGGER foods_search_vector_trigger;
         ALTER TABLE food_aliases ENABLE TRIGGER food_aliases_search_vector_trigger;",
    )
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "SELECT refresh_food_search_vector(id)
         FROM foods
         WHERE source = $1 AND NOT is_archived",
    )
    .bind(source)
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;
    println!(
        "imported {} foods and {} nutrient values from {}",
        source_ids.len(),
        values.len(),
        source
    );
    Ok(())
}
