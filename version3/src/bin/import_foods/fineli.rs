use std::{
    collections::{BTreeMap, HashMap},
    error::Error,
    fs, io,
    path::{Path, PathBuf},
};

use csv::{ReaderBuilder, StringRecord};
use encoding_rs::WINDOWS_1252;
use serde_json::{Map, Value, json};

use super::{
    model::{Alias, Food, FoodImport, SOURCE_FINELI, SourceMapping, data_error},
    nutrients::{definition, fineli_code},
};

pub fn read_fineli(directory: &Path) -> Result<FoodImport, Box<dyn Error>> {
    let component_names = read_code_names(directory.join("eufdname_EN.csv"))?;
    let component_rows = read_latin_csv(directory.join("component.csv"))?;
    let mut nutrients = BTreeMap::new();
    let mut mappings = BTreeMap::new();
    let mut component_info = HashMap::new();

    for row in &component_rows.rows {
        let source_key = field(&component_rows.headers, row, "EUFDNAME")?;
        let source_unit = field(&component_rows.headers, row, "COMPUNIT")?;
        let Some(canonical) = fineli_code(source_key) else {
            continue;
        };
        let (unit, factor) = fineli_unit(source_unit)?;
        let definition = definition(canonical)
            .ok_or_else(|| data_error(format!("unknown canonical nutrient {canonical}")))?;
        if definition.unit != unit {
            return Err(data_error(format!(
                "Fineli nutrient {source_key} uses {unit}, expected {}",
                definition.unit
            ))
            .into());
        }
        let source_name = component_names
            .get(source_key)
            .cloned()
            .unwrap_or_else(|| source_key.to_owned());
        nutrients.entry(canonical.to_owned()).or_insert(definition);
        mappings.insert(
            source_key.to_owned(),
            SourceMapping {
                source_key: source_key.to_owned(),
                source_name,
                nutrient_code: canonical.to_owned(),
                source_unit: source_unit.to_owned(),
            },
        );
        component_info.insert(source_key.to_owned(), (canonical.to_owned(), factor));
    }

    let food_rows = read_latin_csv(directory.join("food.csv"))?;
    let mut foods = BTreeMap::new();
    for row in &food_rows.rows {
        let source_id = field(&food_rows.headers, row, "FOODID")?.to_owned();
        let source_name = field(&food_rows.headers, row, "FOODNAME")?;
        let display_name = normalize_name(source_name);
        let source_data = json!({
            "release": "20.0",
            "display_name": source_name,
            "names": Map::<String, Value>::new(),
            "food_type": field(&food_rows.headers, row, "FOODTYPE")?,
            "process": field(&food_rows.headers, row, "PROCESS")?,
            "edible_portion_percent": field(&food_rows.headers, row, "EDPORT")?,
            "ingredient_class": field(&food_rows.headers, row, "IGCLASS")?,
            "ingredient_class_parent": field(&food_rows.headers, row, "IGCLASSP")?,
            "food_use_class": field(&food_rows.headers, row, "FUCLASS")?,
            "food_use_class_parent": field(&food_rows.headers, row, "FUCLASSP")?,
            "nutrients": Map::<String, Value>::new(),
        });
        foods.insert(
            source_id.clone(),
            Food {
                source_id,
                display_name,
                brand: String::new(),
                source_data,
                aliases: Vec::new(),
                nutrients: BTreeMap::new(),
            },
        );
    }

    for (filename, locale) in [
        ("foodname_FI.csv", "fi"),
        ("foodname_SV.csv", "sv"),
        ("foodname_EN.csv", "en"),
        ("foodname_TX.csv", "sci"),
    ] {
        let names = read_latin_csv(directory.join(filename))?;
        for row in &names.rows {
            let source_id = field(&names.headers, row, "FOODID")?;
            let source_name = field(&names.headers, row, "FOODNAME")?.trim();
            if let Some(food) = foods.get_mut(source_id)
                && !source_name.is_empty()
            {
                let names = food
                    .source_data
                    .get_mut("names")
                    .and_then(Value::as_object_mut)
                    .expect("Fineli source data names must be an object");
                names.insert(locale.to_owned(), Value::String(source_name.to_owned()));
                let name = normalize_name(source_name);
                food.aliases.push(Alias {
                    name: name.clone(),
                    locale: locale.to_owned(),
                });
                if locale == "fi" {
                    food.display_name = name;
                }
            }
        }
    }

    let value_rows = read_latin_csv(directory.join("component_value.csv"))?;
    for row in &value_rows.rows {
        let source_id = field(&value_rows.headers, row, "FOODID")?;
        let source_key = field(&value_rows.headers, row, "EUFDNAME")?;
        let raw_value = field(&value_rows.headers, row, "BESTLOC")?;
        if raw_value.trim().is_empty() {
            continue;
        }
        let value = parse_decimal(raw_value)?;
        if !value.is_finite() || value < 0.0 {
            continue;
        }
        let Some((canonical, factor)) = component_info.get(source_key) else {
            continue;
        };
        let Some(food) = foods.get_mut(source_id) else {
            return Err(data_error(format!("unknown Fineli food {source_id}")).into());
        };
        food.nutrients.insert(canonical.clone(), value * factor);
        let raw_nutrients = food
            .source_data
            .get_mut("nutrients")
            .and_then(Value::as_object_mut)
            .expect("Fineli source data nutrients must be an object");
        raw_nutrients.insert(
            source_key.to_owned(),
            json!({
                "value": raw_value,
                "acquisition_type": field(&value_rows.headers, row, "ACQTYPE")?,
                "method_type": field(&value_rows.headers, row, "METHTYPE")?,
                "method": field(&value_rows.headers, row, "METHIND")?,
            }),
        );
    }

    Ok(FoodImport {
        source: SOURCE_FINELI,
        nutrients,
        mappings,
        foods: foods
            .into_values()
            .filter(|food| !food.nutrients.is_empty())
            .collect(),
    })
}

struct CsvRows {
    headers: StringRecord,
    rows: Vec<StringRecord>,
}

fn read_latin_csv(path: PathBuf) -> Result<CsvRows, Box<dyn Error>> {
    let bytes = fs::read(&path)?;
    let (text, _, had_errors) = WINDOWS_1252.decode(&bytes);
    if had_errors {
        return Err(data_error(format!("could not decode {}", path.display())).into());
    }
    let mut reader = ReaderBuilder::new()
        .delimiter(b';')
        .from_reader(text.as_bytes());
    let headers = reader.headers()?.clone();
    let rows = reader.records().collect::<Result<Vec<_>, _>>()?;
    Ok(CsvRows { headers, rows })
}

fn read_code_names(path: PathBuf) -> Result<HashMap<String, String>, Box<dyn Error>> {
    let rows = read_latin_csv(path)?;
    let mut names = HashMap::new();
    for row in &rows.rows {
        names.insert(
            field(&rows.headers, row, "THSCODE")?.to_owned(),
            field(&rows.headers, row, "DESCRIPT")?.to_owned(),
        );
    }
    Ok(names)
}

fn field<'a>(
    headers: &StringRecord,
    row: &'a StringRecord,
    name: &str,
) -> Result<&'a str, io::Error> {
    let index = headers
        .iter()
        .position(|header| header == name)
        .ok_or_else(|| data_error(format!("missing column {name}")))?;
    row.get(index)
        .ok_or_else(|| data_error(format!("missing value for column {name}")))
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

fn parse_decimal(value: &str) -> Result<f64, io::Error> {
    value
        .replace(',', ".")
        .parse()
        .map_err(|_| data_error(format!("invalid number {value:?}")))
}

fn fineli_unit(unit: &str) -> Result<(&'static str, f64), io::Error> {
    match unit {
        "KJ" => Ok(("kJ", 1.0)),
        "G" => Ok(("g", 1.0)),
        "MG" => Ok(("g", 0.001)),
        "UG" => Ok(("g", 0.000_001)),
        _ => Err(data_error(format!("unsupported Fineli unit {unit}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_decimal_commas() {
        assert_eq!(parse_decimal("12,340").unwrap(), 12.34);
    }

    #[test]
    fn normalizes_fineli_names_without_losing_known_abbreviations() {
        assert_eq!(normalize_name("MAITO, RASVATON"), "Maito, rasvaton");
        assert_eq!(
            normalize_name("MAITO, UHT, D-VITAMIINI"),
            "Maito, UHT, D-vitamiini"
        );
    }
}
