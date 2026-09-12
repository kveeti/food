use std::collections::{BTreeMap, HashMap};

use serde_json::Value;

pub const SOURCE_FINELI: &str = "fineli";
pub const SOURCE_OFF: &str = "open_food_facts";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanonicalUnit {
    Grams,
    Kilojoules,
}

#[derive(Debug)]
pub struct NutrientRegistry {
    units: HashMap<String, CanonicalUnit>,
}

impl NutrientRegistry {
    pub fn new(units: HashMap<String, CanonicalUnit>) -> Self {
        Self { units }
    }

    pub fn unit(&self, code: &str) -> Option<CanonicalUnit> {
        self.units.get(code).copied()
    }

    pub fn contains(&self, code: &str) -> bool {
        self.units.contains_key(code)
    }
}

#[derive(Debug)]
pub struct FoodName {
    pub name: String,
    pub locale: Option<String>,
}

#[derive(Debug)]
pub struct Food {
    pub source_id: String,
    pub display_name: String,
    pub brand: Option<String>,
    pub basis_unit: &'static str,
    pub source_data: Value,
    pub names: Vec<FoodName>,
    pub nutrients: BTreeMap<String, f64>,
}

#[derive(Default)]
pub struct ImportBatch {
    pub touched_source_ids: Vec<String>,
    pub foods: Vec<Food>,
}

#[derive(Default)]
pub struct ImportReport {
    pub scanned_foods: u64,
    pub imported_foods: u64,
    pub imported_values: u64,
    pub mapped_values: BTreeMap<String, u64>,
    pub skipped_basis: u64,
    pub skipped_names: u64,
    pub skipped_nutrients: u64,
    pub ignored_values: BTreeMap<String, u64>,
    pub invalid_values: BTreeMap<String, u64>,
    pub incompatible_values: BTreeMap<String, u64>,
    pub unknown_values: BTreeMap<String, u64>,
}

impl ImportReport {
    pub fn mapped(&mut self, source_key: &str, canonical_code: &str) {
        *self
            .mapped_values
            .entry(format!("{source_key} -> {canonical_code}"))
            .or_default() += 1;
    }

    pub fn ignored(&mut self, key: impl Into<String>) {
        *self.ignored_values.entry(key.into()).or_default() += 1;
    }

    pub fn invalid(&mut self, key: impl Into<String>) {
        *self.invalid_values.entry(key.into()).or_default() += 1;
    }

    pub fn unknown(&mut self, key: impl Into<String>) {
        *self.unknown_values.entry(key.into()).or_default() += 1;
    }

    pub fn incompatible(&mut self, key: impl Into<String>) {
        *self.incompatible_values.entry(key.into()).or_default() += 1;
    }
}
