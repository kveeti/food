use std::{collections::BTreeMap, io};

use serde_json::Value;

pub const SOURCE_FINELI: &str = "fineli";
pub const SOURCE_OFF: &str = "open_food_facts";

#[derive(Debug, Clone)]
pub struct NutrientDef {
    pub code: String,
    pub display_name: String,
    pub unit: String,
    pub category: String,
    pub display_order: i32,
}

#[derive(Debug, Clone)]
pub struct SourceMapping {
    pub source_key: String,
    pub nutrient_code: String,
    pub source_unit: String,
}

#[derive(Debug)]
pub struct Alias {
    pub name: String,
    pub locale: String,
}

#[derive(Debug)]
pub struct Food {
    pub source_id: String,
    pub display_name: String,
    pub brand: String,
    pub source_data: Value,
    pub aliases: Vec<Alias>,
    pub nutrients: BTreeMap<String, f64>,
}

#[derive(Debug)]
pub struct FoodImport {
    pub source: &'static str,
    pub nutrients: BTreeMap<String, NutrientDef>,
    pub mappings: BTreeMap<String, SourceMapping>,
    pub foods: Vec<Food>,
}

pub fn data_error(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

pub fn nutrient_order(code: &str, fallback: i32) -> i32 {
    match code {
        "energy" => 0,
        "fat" => 10,
        "saturated-fat" => 11,
        "carbohydrate" => 20,
        "sugars" => 21,
        "fibre" => 30,
        "protein" => 40,
        "salt" => 50,
        "sodium" => 51,
        _ => 1000 + fallback,
    }
}
