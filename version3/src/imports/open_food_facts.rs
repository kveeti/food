use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    io::{BufRead, BufReader, Read},
};

use serde::Deserialize;
use serde_json::{Map, Value, json};

use super::{
    model::{Alias, Food, FoodDelta, FoodImport, SOURCE_OFF, SourceMapping, data_error},
    nutrients::{definition, off_code},
};

pub fn read_open_food_facts(reader: impl Read) -> Result<FoodImport, Box<dyn Error>> {
    Ok(read_products(reader, false)?.0)
}

pub fn read_open_food_facts_delta(reader: impl Read) -> Result<FoodDelta, Box<dyn Error>> {
    let (import, changed_source_ids) = read_products(reader, true)?;
    Ok(FoodDelta {
        import,
        changed_source_ids,
    })
}

fn read_products(
    reader: impl Read,
    delta: bool,
) -> Result<(FoodImport, Vec<String>), Box<dyn Error>> {
    let reader = BufReader::with_capacity(1024 * 1024, reader);
    let mut nutrients = BTreeMap::new();
    let mut mappings = BTreeMap::new();
    let mut foods = Vec::new();
    let mut unknown_nutrients = BTreeSet::new();
    let mut scanned = 0;
    let mut skipped_basis = 0;
    let mut changed_source_ids = BTreeSet::new();

    for (line_index, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        scanned += 1;
        if !delta && !line.contains("\"en:finland\"") {
            continue;
        }
        let product: OffProduct = serde_json::from_str(&line).map_err(|error| {
            data_error(format!(
                "invalid Open Food Facts JSON on line {}: {error}",
                line_index + 1
            ))
        })?;

        let Some(source_id) = text(&product.code) else {
            continue;
        };
        if delta {
            changed_source_ids.insert(source_id.clone());
        }
        if !product
            .countries_tags
            .iter()
            .any(|country| country == "en:finland")
        {
            continue;
        }
        let Some(display_name) = text(&product.product_name) else {
            continue;
        };
        let source_basis = text(&product.nutrition_data_per);
        let aggregate = match product
            .nutrition
            .and_then(|nutrition| nutrition.aggregated_set)
        {
            Some(aggregate) => aggregate,
            None => {
                let Some(source_basis) = source_basis.as_deref() else {
                    skipped_basis += 1;
                    continue;
                };
                let Some(aggregate) = legacy_aggregate(product.nutriments.as_ref(), source_basis)
                else {
                    continue;
                };
                aggregate
            }
        };
        let Some(aggregate_basis) = text(&aggregate.per) else {
            skipped_basis += 1;
            continue;
        };
        if text(&aggregate.preparation).as_deref() != Some("as_sold") {
            skipped_basis += 1;
            continue;
        }
        let basis_unit = match aggregate_basis.as_str() {
            "100g" => "g",
            "100ml" => "ml",
            _ => {
                skipped_basis += 1;
                continue;
            }
        };

        let mut values = BTreeMap::new();
        let mut priorities = BTreeMap::new();
        let mut raw_nutrients = Map::new();
        for (source_key, source_value) in aggregate.nutrients {
            let Some(source_per) = text(&source_value.source_per) else {
                continue;
            };
            let Some(value) = number(&source_value.value) else {
                continue;
            };
            if source_per != aggregate_basis || !value.is_finite() || value < 0.0 {
                continue;
            }
            let Some(source_unit) = text(&source_value.unit) else {
                continue;
            };
            raw_nutrients.insert(
                source_key.clone(),
                json!({
                    "value": value,
                    "unit": source_unit,
                    "source_per": source_per,
                }),
            );

            let Some(canonical_code) = off_code(&source_key) else {
                unknown_nutrients.insert(source_key);
                continue;
            };
            let definition = definition(canonical_code).ok_or_else(|| {
                data_error(format!("unknown canonical nutrient {canonical_code}"))
            })?;
            let Some(factor) = unit_factor(&source_unit, &definition.unit) else {
                continue;
            };
            let canonical_value = value * factor;
            if !canonical_value.is_finite() {
                continue;
            }

            nutrients
                .entry(canonical_code.to_owned())
                .or_insert(definition);
            mappings
                .entry(source_key.clone())
                .or_insert_with(|| SourceMapping {
                    source_key: source_key.clone(),
                    source_name: source_key.clone(),
                    nutrient_code: canonical_code.to_owned(),
                    source_unit: source_unit.clone(),
                });

            let priority = nutrient_priority(&source_key);
            if priorities.get(canonical_code).copied().unwrap_or(0) < priority {
                values.insert(canonical_code.to_owned(), canonical_value);
                priorities.insert(canonical_code.to_owned(), priority);
            }
        }
        if values.is_empty() {
            continue;
        }

        let generic_name = text(&product.generic_name).unwrap_or_default();
        let aliases = if generic_name.is_empty() || generic_name == display_name {
            Vec::new()
        } else {
            vec![Alias {
                name: generic_name.clone(),
                locale: String::new(),
            }]
        };
        let source_data = json!({
            "generic_name": generic_name,
            "quantity": product.quantity,
            "countries_tags": product.countries_tags,
            "nutrition_data_per": source_basis,
            "product_quantity": product.product_quantity,
            "product_quantity_unit": product.product_quantity_unit,
            "serving_quantity": product.serving_quantity,
            "serving_quantity_unit": product.serving_quantity_unit,
            "nutrition": {
                "preparation": "as_sold",
                "per": aggregate_basis,
                "nutrients": raw_nutrients,
            },
        });
        foods.push(Food {
            source_id,
            display_name,
            brand: text(&product.brands).unwrap_or_default(),
            basis_unit: basis_unit.to_owned(),
            source_data,
            aliases,
            nutrients: values,
        });
    }

    if scanned == 0 {
        return Err(data_error("Open Food Facts input was empty").into());
    }
    if foods.is_empty() && !delta {
        return Err(data_error("Open Food Facts input contained no importable foods").into());
    }
    let gram_foods = foods.iter().filter(|food| food.basis_unit == "g").count();
    let millilitre_foods = foods.len() - gram_foods;
    eprintln!(
        "scanned {scanned} Open Food Facts products; selected {gram_foods} gram and {millilitre_foods} millilitre foods; skipped {skipped_basis} Finnish products with an unusable nutrition basis; ignored {} unknown nutrient keys",
        unknown_nutrients.len()
    );
    Ok((
        FoodImport {
            source: SOURCE_OFF,
            nutrients,
            mappings,
            foods,
        },
        changed_source_ids.into_iter().collect(),
    ))
}

#[derive(Deserialize)]
struct OffProduct {
    #[serde(default)]
    code: Value,
    #[serde(default)]
    product_name: Value,
    #[serde(default)]
    brands: Value,
    #[serde(default)]
    generic_name: Value,
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
    #[serde(default)]
    serving_quantity: Value,
    #[serde(default)]
    serving_quantity_unit: Value,
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

fn legacy_aggregate(
    nutriments: Option<&Map<String, Value>>,
    basis: &str,
) -> Option<OffNutrientSet> {
    if !matches!(basis, "100g" | "100ml") {
        return None;
    }
    let nutrients = nutriments?
        .iter()
        .filter_map(|(field, value)| {
            let source_key = field.strip_suffix("_100g")?;
            number(value)?;
            let unit = match source_key {
                "energy-kcal" => "kcal",
                "energy" | "energy-kj" | "energy-from-fat" => "kJ",
                _ => "g",
            };
            Some((
                source_key.to_owned(),
                OffNutrientValue {
                    value: value.clone(),
                    unit: Value::String(unit.to_owned()),
                    source_per: Value::String(basis.to_owned()),
                },
            ))
        })
        .collect();
    Some(OffNutrientSet {
        per: Value::String(basis.to_owned()),
        preparation: Value::String("as_sold".to_owned()),
        nutrients,
    })
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
        .or_else(|| value.as_str()?.trim().parse().ok())
}

fn unit_factor(source: &str, canonical: &str) -> Option<f64> {
    match (source, canonical) {
        ("kJ", "kJ") => Some(1.0),
        ("kcal", "kJ") => Some(4.184),
        ("kg", "g") => Some(1000.0),
        ("g", "g") => Some(1.0),
        ("mg", "g") => Some(0.001),
        ("µg" | "μg" | "ug" | "mcg", "g") => Some(0.000_001),
        _ => None,
    }
}

fn nutrient_priority(source_key: &str) -> u8 {
    match source_key {
        "energy-kj" => 3,
        "energy" => 2,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_explicit_as_sold_aggregate_bases() {
        let input = concat!(
            r#"{"code":"1","product_name":"Gram food","brands":"Brand","generic_name":"Alias","countries_tags":["en:finland"],"nutrition_data_per":"100g","nutrition":{"aggregated_set":{"per":"100g","preparation":"as_sold","nutrients":{"energy-kcal":{"value":100,"unit":"kcal","source_per":"100g"},"proteins":{"value":2,"unit":"g","source_per":"100g"},"vitamin-c":{"value":30,"unit":"mg","source_per":"100g"}}}}}"#,
            "\n",
            r#"{"code":"2","product_name":"Drink","countries_tags":["en:finland"],"nutrition_data_per":"100ml","nutrition":{"aggregated_set":{"per":"100ml","preparation":"as_sold","nutrients":{"carbohydrates":{"value":4.5,"unit":"g","source_per":"100ml"}}}}}"#,
            "\n",
            r#"{"code":"3","product_name":"Serving food","countries_tags":["en:finland"],"nutrition_data_per":"serving","nutrition":{"aggregated_set":{"per":"serving","preparation":"as_sold","nutrients":{"fat":{"value":1,"unit":"g","source_per":"serving"}}}}}"#,
            "\n",
            r#"{"code":"4","product_name":"Aggregate wins","countries_tags":["en:finland"],"nutrition_data_per":"100g","nutrition":{"aggregated_set":{"per":"100ml","preparation":"as_sold","nutrients":{"fat":{"value":1,"unit":"g","source_per":"100ml"}}}}}"#,
            "\n",
            r#"{"code":"5","product_name":"Prepared","countries_tags":["en:finland"],"nutrition_data_per":"100g","nutrition":{"aggregated_set":{"per":"100g","preparation":"prepared","nutrients":{"fat":{"value":1,"unit":"g","source_per":"100g"}}}}}"#,
            "\n",
            r#"{"code":"6","product_name":"Swedish food","countries_tags":["en:sweden"],"nutrition_data_per":"100g","nutrition":{"aggregated_set":{"per":"100g","preparation":"as_sold","nutrients":{"fat":{"value":1,"unit":"g","source_per":"100g"}}}}}"#,
            "\n",
            r#"{"code":"7","product_name":"Legacy food","countries_tags":["en:finland"],"nutrition_data_per":"100g","nutrition":null,"nutriments":{"energy_100g":209,"energy_unit":"kcal","energy-kcal_100g":50,"energy-kcal_unit":"kcal","proteins_100g":0.9,"proteins_unit":"g"}}"#,
            "\n",
        );
        let import = read_open_food_facts(input.as_bytes()).unwrap();

        assert_eq!(import.foods.len(), 4);
        assert_eq!(import.foods[0].basis_unit, "g");
        assert_eq!(import.foods[0].display_name, "Gram food");
        assert!((import.foods[0].nutrients["energy"] - 418.4).abs() < 0.000_001);
        assert_eq!(import.foods[0].nutrients["protein"], 2.0);
        assert_eq!(import.foods[0].nutrients["vitamin-c"], 0.03);
        assert_eq!(import.foods[1].basis_unit, "ml");
        assert_eq!(import.foods[1].nutrients["carbohydrate"], 4.5);
        assert_eq!(import.foods[2].basis_unit, "ml");
        assert_eq!(import.foods[2].nutrients["fat"], 1.0);
        assert_eq!(import.foods[3].basis_unit, "g");
        assert_eq!(import.foods[3].nutrients["energy"], 209.0);
        assert_eq!(import.foods[3].nutrients["protein"], 0.9);
        assert_eq!(import.nutrients["protein"].display_name, "Protein");
        assert_eq!(import.mappings["proteins"].source_name, "proteins");
    }

    #[test]
    fn delta_keeps_all_changed_codes_but_imports_only_finnish_products() {
        let input = concat!(
            r#"{"code":"1","product_name":"Finnish","countries_tags":["en:finland"],"nutrition_data_per":"100g","nutriments":{"proteins_100g":2}}"#,
            "\n",
            r#"{"code":"2","product_name":"Removed","countries_tags":["en:sweden"],"nutrition_data_per":"100g","nutriments":{"proteins_100g":3}}"#,
            "\n",
        );
        let delta = read_open_food_facts_delta(input.as_bytes()).unwrap();

        assert_eq!(delta.changed_source_ids, ["1", "2"]);
        assert_eq!(delta.import.foods.len(), 1);
        assert_eq!(delta.import.foods[0].source_id, "1");
    }
}
