use std::{collections::BTreeMap, fs, path::Path};

use anyhow::{Context, Result, bail};
use csv::{ReaderBuilder, StringRecord};
use encoding_rs::WINDOWS_1252;
use serde_json::json;

use crate::{
    database::ImportWriter,
    model::{CanonicalUnit, Food, FoodName, ImportBatch, ImportReport, NutrientRegistry},
    nutrients::fineli_code,
};

const BATCH_SIZE: usize = 500;

struct Component {
    canonical_code: &'static str,
    factor: f64,
}

struct PendingFood {
    display_name: String,
    brand: Option<String>,
    source_data: serde_json::Value,
    names: Vec<FoodName>,
    nutrients: BTreeMap<String, f64>,
}

pub async fn import(directory: &Path, writer: &mut ImportWriter<'_>) -> Result<ImportReport> {
    let (scanned_foods, foods, mapped_values) = read_fineli(directory, writer.registry())?;
    let report = ImportReport {
        scanned_foods,
        imported_foods: foods.len() as u64,
        mapped_values,
        imported_values: foods.iter().map(|food| food.nutrients.len() as u64).sum(),
        ..ImportReport::default()
    };

    let mut batch = ImportBatch::default();
    for food in foods {
        batch.foods.push(food);
        if batch.foods.len() == BATCH_SIZE {
            writer.write_batch(batch).await?;
            batch = ImportBatch::default();
        }
    }
    writer.write_batch(batch).await?;
    Ok(report)
}

fn read_fineli(
    directory: &Path,
    registry: &NutrientRegistry,
) -> Result<(u64, Vec<Food>, BTreeMap<String, u64>)> {
    let mut components = BTreeMap::new();
    for_each_row(directory.join("component.csv"), |headers, row| {
        let source_code = field(headers, row, "EUFDNAME")?;
        let source_unit = field(headers, row, "COMPUNIT")?;
        let canonical_code = fineli_code(source_code)
            .with_context(|| format!("Fineli component {source_code} has no reviewed mapping"))?;
        let (unit, factor) = fineli_unit(source_unit)?;
        let expected = registry
            .unit(canonical_code)
            .with_context(|| format!("canonical nutrient {canonical_code} is missing"))?;
        if unit != expected {
            bail!(
                "Fineli component {source_code} maps to {canonical_code}, but its unit {source_unit} is incompatible"
            );
        }
        components.insert(
            source_code.to_owned(),
            Component {
                canonical_code,
                factor,
            },
        );
        Ok(())
    })?;
    if components.len() != 74 {
        bail!(
            "expected 74 mapped Fineli components, found {}",
            components.len()
        );
    }

    let mut foods = BTreeMap::new();
    for_each_row(directory.join("food.csv"), |headers, row| {
        let source_id = field(headers, row, "FOODID")?.trim();
        if source_id.is_empty() {
            bail!("Fineli food has an empty FOODID");
        }
        let display_name = normalize_name(field(headers, row, "FOODNAME")?);
        if display_name.is_empty() {
            bail!("Fineli food {source_id} has an empty name");
        }
        let edible_portion_percent = optional_decimal(field(headers, row, "EDPORT")?)?;
        let source_data = json!({
            "food_type": optional_text(field(headers, row, "FOODTYPE")?),
            "process": optional_text(field(headers, row, "PROCESS")?),
            "edible_portion_percent": edible_portion_percent,
            "ingredient_class": optional_text(field(headers, row, "IGCLASS")?),
            "ingredient_class_parent": optional_text(field(headers, row, "IGCLASSP")?),
            "food_use_class": optional_text(field(headers, row, "FUCLASS")?),
            "food_use_class_parent": optional_text(field(headers, row, "FUCLASSP")?),
        });
        foods.insert(
            source_id.to_owned(),
            PendingFood {
                display_name,
                brand: None,
                source_data,
                names: Vec::new(),
                nutrients: BTreeMap::new(),
            },
        );
        Ok(())
    })?;

    let scanned_foods = foods.len() as u64;

    for (filename, locale) in [
        ("foodname_FI.csv", Some("fi")),
        ("foodname_SV.csv", Some("sv")),
        ("foodname_EN.csv", Some("en")),
        ("foodname_TX.csv", None),
    ] {
        for_each_row(directory.join(filename), |headers, row| {
            let source_id = field(headers, row, "FOODID")?;
            let Some(food) = foods.get_mut(source_id) else {
                return Ok(());
            };
            let name = normalize_name(field(headers, row, "FOODNAME")?);
            if name.is_empty() {
                return Ok(());
            }
            if locale == Some("fi") {
                food.display_name.clone_from(&name);
            }
            if !food.names.iter().any(|existing| {
                existing.locale.as_deref() == locale && existing.name.eq_ignore_ascii_case(&name)
            }) {
                food.names.push(FoodName {
                    name,
                    locale: locale.map(str::to_owned),
                });
            }
            Ok(())
        })?;
    }

    let mut mapped_values = BTreeMap::new();
    for_each_row(directory.join("component_value.csv"), |headers, row| {
        let raw_value = field(headers, row, "BESTLOC")?.trim();
        if raw_value.is_empty() {
            return Ok(());
        }
        let value = parse_decimal(raw_value)?;
        if !value.is_finite() || value < 0.0 {
            return Ok(());
        }
        let source_code = field(headers, row, "EUFDNAME")?;
        let component = components
            .get(source_code)
            .with_context(|| format!("Fineli value uses unknown component {source_code}"))?;
        let source_id = field(headers, row, "FOODID")?;
        let food = foods
            .get_mut(source_id)
            .with_context(|| format!("Fineli value uses unknown food {source_id}"))?;
        food.nutrients.insert(
            component.canonical_code.to_owned(),
            value * component.factor,
        );
        *mapped_values
            .entry(format!("{source_code} -> {}", component.canonical_code))
            .or_default() += 1;
        Ok(())
    })?;

    Ok((
        scanned_foods,
        foods
            .into_iter()
            .filter_map(|(source_id, food)| {
                (!food.nutrients.is_empty()).then_some(Food {
                    source_id,
                    display_name: food.display_name,
                    brand: food.brand,
                    basis_unit: "g",
                    source_data: food.source_data,
                    names: food.names,
                    nutrients: food.nutrients,
                })
            })
            .collect(),
        mapped_values,
    ))
}

fn for_each_row(
    path: impl AsRef<Path>,
    mut visit: impl FnMut(&StringRecord, &StringRecord) -> Result<()>,
) -> Result<()> {
    let path = path.as_ref();
    let bytes = fs::read(path).with_context(|| format!("could not read {}", path.display()))?;
    let (text, _, had_errors) = WINDOWS_1252.decode(&bytes);
    if had_errors {
        bail!("could not decode {} as Windows-1252", path.display());
    }
    let mut reader = ReaderBuilder::new()
        .delimiter(b';')
        .from_reader(text.as_bytes());
    let headers = reader.headers()?.clone();
    for row in reader.records() {
        visit(&headers, &row?)?;
    }
    Ok(())
}

fn field<'a>(headers: &StringRecord, row: &'a StringRecord, name: &str) -> Result<&'a str> {
    let index = headers
        .iter()
        .position(|header| header == name)
        .with_context(|| format!("missing column {name}"))?;
    row.get(index)
        .with_context(|| format!("missing value for column {name}"))
}

fn optional_text(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

fn optional_decimal(value: &str) -> Result<Option<f64>> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    Ok(Some(parse_decimal(value)?))
}

fn parse_decimal(value: &str) -> Result<f64> {
    value
        .replace(',', ".")
        .parse()
        .with_context(|| format!("invalid number {value:?}"))
}

fn fineli_unit(unit: &str) -> Result<(CanonicalUnit, f64)> {
    match unit {
        "KJ" => Ok((CanonicalUnit::Kilojoules, 1.0)),
        "G" => Ok((CanonicalUnit::Grams, 1.0)),
        "MG" => Ok((CanonicalUnit::Grams, 0.001)),
        "UG" => Ok((CanonicalUnit::Grams, 0.000_001)),
        _ => bail!("unsupported Fineli unit {unit}"),
    }
}

fn normalize_name(source: &str) -> String {
    let mut normalized = String::new();
    let mut token = String::new();
    let flush_token = |normalized: &mut String, token: &mut String| {
        if token.is_empty() {
            return;
        }
        match token.as_str() {
            "uht" => normalized.push_str("UHT"),
            "htst" => normalized.push_str("HTST"),
            "epa" => normalized.push_str("EPA"),
            "dha" => normalized.push_str("DHA"),
            "gmo" => normalized.push_str("GMO"),
            "a-vitamiini" => normalized.push_str("A-vitamiini"),
            "b12-vitamiini" => normalized.push_str("B12-vitamiini"),
            "d-vitamiini" => normalized.push_str("D-vitamiini"),
            "d2-vitamiini" => normalized.push_str("D2-vitamiini"),
            "d3-vitamiini" => normalized.push_str("D3-vitamiini"),
            "e-vitamiini" => normalized.push_str("E-vitamiini"),
            _ => normalized.push_str(token),
        }
        token.clear();
    };

    for character in source.trim().to_lowercase().chars() {
        if character.is_alphanumeric() || character == '-' {
            token.push(character);
        } else {
            flush_token(&mut normalized, &mut token);
            normalized.push(character);
        }
    }
    flush_token(&mut normalized, &mut token);

    if let Some((index, character)) = normalized
        .char_indices()
        .find(|(_, character)| character.is_alphabetic())
        && character.is_lowercase()
    {
        let uppercase = character.to_uppercase().collect::<String>();
        normalized.replace_range(index..index + character.len_utf8(), &uppercase);
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_decimal_commas() {
        assert_eq!(parse_decimal("12,340").unwrap(), 12.34);
    }

    #[test]
    fn normalizes_names_without_losing_known_abbreviations() {
        assert_eq!(normalize_name("MAITO, RASVATON"), "Maito, rasvaton");
        assert_eq!(
            normalize_name("MAITO, UHT, D-VITAMIINI"),
            "Maito, UHT, D-vitamiini"
        );
    }
}
