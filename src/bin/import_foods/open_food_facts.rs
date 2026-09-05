use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader, Read},
};

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::{Map, Value, json};

use crate::{
    database::ImportWriter,
    model::{CanonicalUnit, Food, FoodName, ImportBatch, ImportReport, NutrientRegistry},
    nutrients::{ignored_off_key, off_code},
};

const BATCH_SIZE: usize = 500;
const ETHANOL_GRAMS_PER_MILLILITRE: f64 = 0.789;

pub async fn import(reader: impl Read, writer: &mut ImportWriter<'_>) -> Result<ImportReport> {
    let mut report = ImportReport::default();
    let mut batch = ImportBatch::default();
    let reader = BufReader::with_capacity(1024 * 1024, reader);

    for (line_index, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        report.scanned_foods += 1;
        let product: OffProduct = serde_json::from_str(&line)
            .with_context(|| format!("invalid Open Food Facts JSON on line {}", line_index + 1))?;
        let Some(source_id) = text(&product.code) else {
            continue;
        };
        batch.touched_source_ids.push(source_id.clone());
        if let Some(food) = parse_product(source_id, product, writer.registry(), &mut report) {
            report.imported_foods += 1;
            report.imported_values += food.nutrients.len() as u64;
            batch.foods.push(food);
        }

        if batch.touched_source_ids.len() >= BATCH_SIZE {
            writer.write_batch(batch).await?;
            batch = ImportBatch::default();
        }
    }
    writer.write_batch(batch).await?;
    Ok(report)
}

fn parse_product(
    source_id: String,
    product: OffProduct,
    registry: &NutrientRegistry,
    report: &mut ImportReport,
) -> Option<Food> {
    let names = product_names(&product);
    let display_name = names
        .iter()
        .find(|name| name.locale.as_deref() == Some("fi"))
        .or_else(|| {
            let primary_locale = text(&product.lang);
            names
                .iter()
                .find(|name| name.locale.as_deref() == primary_locale.as_deref())
        })
        .or_else(|| names.first())
        .map(|name| name.name.clone());
    let Some(display_name) = display_name else {
        report.skipped_names += 1;
        return None;
    };

    let source_basis = text(&product.nutrition_data_per);
    let modern = product
        .nutrition
        .as_ref()
        .and_then(|nutrition| nutrition.aggregated_set.as_ref());
    let (basis, source_values) = if let Some(aggregate) = modern {
        let Some(aggregate_basis) = text(&aggregate.per) else {
            report.skipped_basis += 1;
            return None;
        };
        if text(&aggregate.preparation).as_deref() != Some("as_sold") {
            report.skipped_basis += 1;
            return None;
        }
        let Some(basis) = basis(&aggregate_basis) else {
            report.skipped_basis += 1;
            return None;
        };
        let values = aggregate
            .nutrients
            .iter()
            .filter_map(|(key, value)| {
                let source_per = text(&value.source_per)?;
                if source_per != aggregate_basis {
                    report.invalid(format!("{key} [source_per={source_per}]"));
                    return None;
                }
                Some((key.as_str(), number(&value.value)?, text(&value.unit)))
            })
            .collect::<Vec<_>>();
        (basis, values)
    } else {
        let Some(source_basis) = source_basis.as_deref() else {
            report.skipped_basis += 1;
            return None;
        };
        let Some(basis) = basis(source_basis) else {
            report.skipped_basis += 1;
            return None;
        };
        let Some(nutriments) = product.nutriments.as_ref() else {
            report.skipped_nutrients += 1;
            return None;
        };
        let values = nutriments
            .iter()
            .filter_map(|(field, value)| {
                let source_key = field.strip_suffix("_100g")?;
                let value = number(value)?;
                let unit = nutriments.get(&format!("{source_key}_unit")).and_then(text);
                Some((source_key, value, unit))
            })
            .collect::<Vec<_>>();
        (basis, values)
    };

    let mut nutrients = BTreeMap::new();
    let mut priorities = BTreeMap::new();
    for (source_key, value, source_unit) in source_values {
        if !value.is_finite() || value < 0.0 {
            report.invalid(source_key);
            continue;
        }
        let Some(canonical_code) = off_code(source_key, registry) else {
            if ignored_off_key(source_key) {
                report.ignored(source_key);
            } else {
                report.unknown(source_key);
            }
            continue;
        };
        let canonical_unit = registry.unit(&canonical_code)?;
        let source_unit = source_unit.as_deref().unwrap_or("<missing>");
        let Some(factor) = unit_factor(source_unit, canonical_unit, &canonical_code, basis) else {
            report.incompatible(format!("{source_key} [{source_unit}] on 100{basis}"));
            continue;
        };
        let canonical_value = value * factor;
        if !canonical_value.is_finite() {
            report.incompatible(format!("{source_key} [{source_unit}] on 100{basis}"));
            continue;
        }
        report.mapped(source_key, &canonical_code);
        let priority = nutrient_priority(source_key);
        if priorities.get(&canonical_code).copied().unwrap_or(0) < priority {
            nutrients.insert(canonical_code.clone(), canonical_value);
            priorities.insert(canonical_code, priority);
        }
    }
    if nutrients.is_empty() {
        report.skipped_nutrients += 1;
        return None;
    }

    let source_data = json!({
        "countries_tags": product.countries_tags,
        "quantity": text(&product.quantity),
        "product_quantity": number(&product.product_quantity),
        "product_quantity_unit": text(&product.product_quantity_unit),
    });
    Some(Food {
        source_id,
        display_name,
        brand: text(&product.brands),
        basis_unit: basis,
        source_data,
        names,
        nutrients,
    })
}

fn product_names(product: &OffProduct) -> Vec<FoodName> {
    let primary_locale = text(&product.lang).map(|locale| locale.to_lowercase());
    let mut names = Vec::new();
    for (value, locale) in [
        (&product.product_name_fi, Some("fi".to_owned())),
        (&product.product_name_sv, Some("sv".to_owned())),
        (&product.product_name_en, Some("en".to_owned())),
        (&product.product_name, primary_locale.clone()),
        (&product.generic_name_fi, Some("fi".to_owned())),
        (&product.generic_name_sv, Some("sv".to_owned())),
        (&product.generic_name_en, Some("en".to_owned())),
        (&product.generic_name, primary_locale),
    ] {
        let Some(name) = text(value) else {
            continue;
        };
        if !names.iter().any(|existing: &FoodName| {
            existing.locale == locale && existing.name.to_lowercase() == name.to_lowercase()
        }) {
            names.push(FoodName { name, locale });
        }
    }
    names
}

#[derive(Deserialize)]
struct OffProduct {
    #[serde(default)]
    code: Value,
    #[serde(default)]
    product_name: Value,
    #[serde(default)]
    product_name_fi: Value,
    #[serde(default)]
    product_name_sv: Value,
    #[serde(default)]
    product_name_en: Value,
    #[serde(default)]
    generic_name: Value,
    #[serde(default)]
    generic_name_fi: Value,
    #[serde(default)]
    generic_name_sv: Value,
    #[serde(default)]
    generic_name_en: Value,
    #[serde(default)]
    lang: Value,
    #[serde(default)]
    brands: Value,
    #[serde(default)]
    countries_tags: Vec<String>,
    #[serde(default)]
    nutrition_data_per: Value,
    nutrition: Option<OffNutrition>,
    nutriments: Option<Map<String, Value>>,
    #[serde(default)]
    quantity: Value,
    #[serde(default)]
    product_quantity: Value,
    #[serde(default)]
    product_quantity_unit: Value,
}

#[derive(Deserialize)]
struct OffNutrition {
    aggregated_set: Option<OffNutrientSet>,
}

#[derive(Deserialize)]
struct OffNutrientSet {
    #[serde(default)]
    per: Value,
    #[serde(default)]
    preparation: Value,
    #[serde(default)]
    nutrients: BTreeMap<String, OffNutrientValue>,
}

#[derive(Deserialize)]
struct OffNutrientValue {
    #[serde(default)]
    value: Value,
    #[serde(default)]
    unit: Value,
    #[serde(default)]
    source_per: Value,
}

fn basis(source: &str) -> Option<&'static str> {
    match source {
        "100g" => Some("g"),
        "100ml" => Some("ml"),
        _ => None,
    }
}

fn text(value: &Value) -> Option<String> {
    match value {
        Value::String(value) if !value.trim().is_empty() => Some(value.trim().to_owned()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn number(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str()?.trim().replace(',', ".").parse().ok())
}

fn unit_factor(
    source: &str,
    canonical: CanonicalUnit,
    nutrient_code: &str,
    basis_unit: &str,
) -> Option<f64> {
    let normalized = source.trim().to_lowercase();
    match (normalized.as_str(), canonical) {
        ("kj", CanonicalUnit::Kilojoules) => Some(1.0),
        ("kcal", CanonicalUnit::Kilojoules) => Some(4.184),
        ("kg", CanonicalUnit::Grams) => Some(1000.0),
        ("g", CanonicalUnit::Grams) => Some(1.0),
        ("mg", CanonicalUnit::Grams) => Some(0.001),
        ("µg" | "μg" | "ug" | "mcg" | "&#181;g", CanonicalUnit::Grams) => Some(0.000_001),
        ("% vol" | "%vol", CanonicalUnit::Grams)
            if nutrient_code == "alcohol" && basis_unit == "ml" =>
        {
            Some(ETHANOL_GRAMS_PER_MILLILITRE)
        }
        _ => None,
    }
}

fn nutrient_priority(source_key: &str) -> u8 {
    match source_key {
        "energy-kj" => 3,
        "energy-kcal" => 2,
        "energy" => 1,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn registry() -> NutrientRegistry {
        NutrientRegistry::new(HashMap::from([
            ("energy".to_owned(), CanonicalUnit::Kilojoules),
            ("protein".to_owned(), CanonicalUnit::Grams),
            ("fat".to_owned(), CanonicalUnit::Grams),
            ("alcohol".to_owned(), CanonicalUnit::Grams),
            ("vitamin-c".to_owned(), CanonicalUnit::Grams),
        ]))
    }

    fn parse(json: &str) -> Option<Food> {
        let product: OffProduct = serde_json::from_str(json).unwrap();
        let source_id = text(&product.code).unwrap();
        parse_product(
            source_id,
            product,
            &registry(),
            &mut ImportReport::default(),
        )
    }

    #[test]
    fn reads_modern_values_and_prefers_finnish_name() {
        let food = parse(
            r#"{
                "code":"1",
                "product_name":"Milk",
                "product_name_fi":"Maito",
                "lang":"en",
                "nutrition":{"aggregated_set":{
                    "per":"100ml",
                    "preparation":"as_sold",
                    "nutrients":{
                        "energy-kcal":{"value":50,"unit":"kcal","source_per":"100ml"},
                        "proteins":{"value":3.2,"unit":"g","source_per":"100ml"},
                        "vitamin-c":{"value":30,"unit":"mg","source_per":"100ml"}
                    }
                }}
            }"#,
        )
        .unwrap();
        assert_eq!(food.display_name, "Maito");
        assert_eq!(food.basis_unit, "ml");
        assert!((food.nutrients["energy"] - 209.2).abs() < 0.000_001);
        assert_eq!(food.nutrients["protein"], 3.2);
        assert_eq!(food.nutrients["vitamin-c"], 0.03);
    }

    #[test]
    fn legacy_values_use_their_actual_units() {
        let food = parse(
            r#"{
                "code":"1",
                "product_name":"Food",
                "nutrition_data_per":"100g",
                "nutriments":{
                    "energy_100g":100,
                    "energy_unit":"kcal",
                    "proteins_100g":2,
                    "proteins_unit":"g"
                }
            }"#,
        )
        .unwrap();
        assert!((food.nutrients["energy"] - 418.4).abs() < 0.000_001);
        assert_eq!(food.nutrients["protein"], 2.0);
    }

    #[test]
    fn explicit_legacy_energy_wins_over_a_conflicting_generic_value() {
        let food = parse(
            r#"{
                "code":"6420256010815",
                "product_name":"Karkki tvmix",
                "nutrition_data_per":"100g",
                "nutriments":{
                    "energy_100g":1506,
                    "energy_unit":"kcal",
                    "energy-kcal_100g":360,
                    "energy-kcal_unit":"kcal"
                }
            }"#,
        )
        .unwrap();
        assert!((food.nutrients["energy"] - 1506.24).abs() < 0.000_001);
    }

    #[test]
    fn alcohol_volume_percent_only_converts_for_millilitre_basis() {
        assert_eq!(
            unit_factor("% vol", CanonicalUnit::Grams, "alcohol", "ml"),
            Some(ETHANOL_GRAMS_PER_MILLILITRE)
        );
        assert_eq!(
            unit_factor("% vol", CanonicalUnit::Grams, "alcohol", "g"),
            None
        );
    }
}
