use std::{collections::BTreeMap, error::Error, io, io::Read};

use csv::{ReaderBuilder, StringRecord};
use serde_json::{Map, Value};

use super::{
    model::{Alias, Food, FoodImport, SOURCE_OFF, SourceMapping, data_error},
    nutrients::{definition, off_code},
};

pub fn read_open_food_facts(reader: impl Read) -> Result<FoodImport, Box<dyn Error>> {
    let mut reader = ReaderBuilder::new()
        .delimiter(b'\t')
        .flexible(true)
        .from_reader(reader);
    let headers = reader.headers()?.clone();
    let code_index = header_index(&headers, "code")?;
    let name_index = header_index(&headers, "product_name")?;
    let brand_index = header_index(&headers, "brands")?;
    let generic_name_index = header_index(&headers, "generic_name")?;
    let countries_index = header_index(&headers, "countries_tags")?;

    let nutrient_columns = headers
        .iter()
        .enumerate()
        .filter_map(|(index, header)| {
            off_nutrient(header).map(|nutrient| (index, header, nutrient))
        })
        .collect::<Vec<_>>();
    let mut nutrients = BTreeMap::new();
    let mut mappings = BTreeMap::new();
    for (_, source_key, nutrient) in &nutrient_columns {
        let definition = definition(&nutrient.code)
            .ok_or_else(|| data_error(format!("unknown canonical nutrient {}", nutrient.code)))?;
        if definition.unit != nutrient.unit {
            return Err(data_error(format!(
                "OFF nutrient {source_key} uses {}, expected {}",
                nutrient.unit, definition.unit
            ))
            .into());
        }
        nutrients
            .entry(nutrient.code.to_owned())
            .or_insert(definition);
        mappings.insert(
            (*source_key).to_owned(),
            SourceMapping {
                source_key: (*source_key).to_owned(),
                source_name: (*source_key).to_owned(),
                nutrient_code: nutrient.code.to_owned(),
                source_unit: nutrient.source_unit.to_owned(),
            },
        );
    }

    let retained = [
        "url",
        "last_modified_t",
        "generic_name",
        "quantity",
        "brands",
        "categories_tags",
        "countries_tags",
        "ingredients_text",
        "serving_size",
        "data_quality_errors_tags",
    ];
    let retained_indices = retained
        .iter()
        .filter_map(|name| {
            header_index(&headers, name)
                .ok()
                .map(|index| (*name, index))
        })
        .collect::<Vec<_>>();

    let mut foods = Vec::new();
    for row in reader.records() {
        let row = row?;
        let countries = row.get(countries_index).unwrap_or_default();
        if !countries.split(',').any(|country| country == "en:finland") {
            continue;
        }
        let source_id = row.get(code_index).unwrap_or_default().trim();
        let display_name = row.get(name_index).unwrap_or_default().trim();
        if source_id.is_empty() || display_name.is_empty() {
            continue;
        }

        let mut values = BTreeMap::new();
        let mut raw_nutrients = Map::new();
        for (index, source_key, nutrient) in &nutrient_columns {
            let raw = row.get(*index).unwrap_or_default().trim();
            if raw.is_empty() {
                continue;
            }
            let Ok(value) = raw.parse::<f64>() else {
                continue;
            };
            if !value.is_finite() || value < 0.0 {
                continue;
            }
            raw_nutrients.insert((*source_key).to_owned(), Value::String(raw.to_owned()));
            values
                .entry(nutrient.code.to_owned())
                .or_insert(value * nutrient.factor);
        }
        if values.is_empty() {
            continue;
        }

        let mut source_data = Map::new();
        for (name, index) in &retained_indices {
            let value = row.get(*index).unwrap_or_default().trim();
            if !value.is_empty() {
                source_data.insert((*name).to_owned(), Value::String(value.to_owned()));
            }
        }
        source_data.insert("nutrients".to_owned(), Value::Object(raw_nutrients));

        let generic_name = row.get(generic_name_index).unwrap_or_default().trim();
        let aliases = if generic_name.is_empty() || generic_name == display_name {
            Vec::new()
        } else {
            vec![Alias {
                name: generic_name.to_owned(),
                locale: String::new(),
            }]
        };
        foods.push(Food {
            source_id: source_id.to_owned(),
            display_name: display_name.to_owned(),
            brand: row.get(brand_index).unwrap_or_default().trim().to_owned(),
            source_data: Value::Object(source_data),
            aliases,
            nutrients: values,
        });
    }

    Ok(FoodImport {
        source: SOURCE_OFF,
        nutrients,
        mappings,
        foods,
    })
}

fn header_index(headers: &StringRecord, name: &str) -> Result<usize, io::Error> {
    headers
        .iter()
        .position(|header| header == name)
        .ok_or_else(|| data_error(format!("missing column {name}")))
}

struct OffNutrient {
    code: String,
    unit: &'static str,
    source_unit: &'static str,
    factor: f64,
}

fn off_nutrient(header: &str) -> Option<OffNutrient> {
    let key = header.strip_suffix("_100g")?;
    if matches!(
        key,
        "ph" | "fruits-vegetables-legumes"
            | "collagen-meat-protein-ratio"
            | "cocoa"
            | "chlorophyl"
            | "carbon-footprint"
            | "glycemic-index"
            | "water-hardness"
            | "acidity"
    ) {
        return None;
    }

    let (source_code, unit, source_unit, factor) = match key {
        "energy-kj" | "energy" => ("energy", "kJ", "kJ", 1.0),
        "energy-kcal" => ("energy", "kJ", "kcal", 4.184),
        "energy-from-fat" => ("energy-from-fat", "kJ", "kJ", 1.0),
        "proteins" => ("protein", "g", "g", 1.0),
        "carbohydrates" => ("carbohydrate", "g", "g", 1.0),
        "fiber" => ("fibre", "g", "g", 1.0),
        "vitamin-b1" => ("thiamin", "g", "g", 1.0),
        "vitamin-b2" => ("riboflavin", "g", "g", 1.0),
        "vitamin-pp" => ("niacin", "g", "g", 1.0),
        "vitamin-b9" | "folates" => ("folate", "g", "g", 1.0),
        other => (other, "g", "g", 1.0),
    };
    let code = off_code(source_code)?;
    Some(OffNutrient {
        code: code.to_owned(),
        unit,
        source_unit,
        factor,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_finnish_products_and_all_present_nutrients() {
        let input = concat!(
            "code\tproduct_name\tbrands\tgeneric_name\tcountries_tags\tenergy-kcal_100g\tproteins_100g\tvitamin-c_100g\n",
            "1\tFinnish food\tBrand\tAlias\ten:finland\t100\t2\t0.003\n",
            "2\tSwedish food\tBrand\t\ten:sweden\t200\t3\t\n",
        );
        let import = read_open_food_facts(input.as_bytes()).unwrap();

        assert_eq!(import.foods.len(), 1);
        let food = &import.foods[0];
        assert_eq!(food.display_name, "Finnish food");
        assert!((food.nutrients["energy"] - 418.4).abs() < 0.000_001);
        assert_eq!(food.nutrients["protein"], 2.0);
        assert_eq!(food.nutrients["vitamin-c"], 0.003);
        assert_eq!(import.nutrients["protein"].display_name, "Protein");
        assert_eq!(
            import.mappings["proteins_100g"].source_name,
            "proteins_100g"
        );
    }
}
