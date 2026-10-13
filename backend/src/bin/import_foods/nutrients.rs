use crate::model::NutrientRegistry;

pub fn fineli_code(code: &str) -> Option<&'static str> {
    Some(match code {
        "ENERC" => "energy",
        "FAT" => "fat",
        "CHOAVL" => "carbohydrate",
        "CHOCDF" => "carbohydrate-by-difference",
        "PROT" => "protein",
        "ALC" => "alcohol",
        "ASH" => "ash",
        "WATER" => "water",
        "OA" => "organic-acids",
        "SUGOH" => "polyols",
        "SUGAR" => "sugars",
        "FRUS" => "fructose",
        "GALS" => "galactose",
        "GLUS" => "glucose",
        "LACS" => "lactose",
        "MALS" => "maltose",
        "SUCS" => "sucrose",
        "STARCH" => "starch",
        "FIBC" => "fibre",
        "FIBT" => "dietary-fibre",
        "FIBINS" => "insoluble-fibre",
        "PSACNCS" => "soluble-non-cellulosic-polysaccharides",
        "FOL" => "folate",
        "NIAEQ" => "niacin-equivalents",
        "NIA" => "niacin",
        "VITPYRID" => "vitamin-b6",
        "RIBF" => "riboflavin",
        "THIA" => "thiamin",
        "VITA" => "vitamin-a",
        "RETOL" => "retinol",
        "CAROTENS" => "carotenoids",
        "CARTB" => "beta-carotene",
        "VITB12" => "vitamin-b12",
        "VITC" => "vitamin-c",
        "VITD" => "vitamin-d",
        "VITE" => "vitamin-e",
        "VITK" => "vitamin-k",
        "CA" => "calcium",
        "CR" => "chromium",
        "CU" => "copper",
        "FD" => "fluoride",
        "FE" => "iron",
        "ID" => "iodine",
        "K" => "potassium",
        "MG" => "magnesium",
        "MN" => "manganese",
        "MO" => "molybdenum",
        "NA" => "sodium",
        "NACL" => "salt",
        "NT" => "nitrogen",
        "P" => "phosphorus",
        "SE" => "selenium",
        "ZN" => "zinc",
        "FAFRE" => "fatty-acids",
        "FACIDCTG" => "fatty-acids-tag-equivalent",
        "FAPU" => "polyunsaturated-fat",
        "FAMCIS" => "monounsaturated-fat",
        "FASAT" => "saturated-fat",
        "FATRN" => "trans-fat",
        "FAPUN3" => "omega-3-fat",
        "FAPUN6" => "omega-6-fat",
        "FAS18" => "stearic-acid",
        "F16D0T" => "palmitic-acid",
        "F18D1T" => "fatty-acid-18-1",
        "F18D2CN6" => "linoleic-acid",
        "F18D3N3" => "alpha-linolenic-acid",
        "F20D4N6" => "arachidonic-acid",
        "F20D5N3" => "eicosapentaenoic-acid",
        "F22D6N3" => "docosahexaenoic-acid",
        "CHOLE" => "cholesterol",
        "STERT" => "sterols",
        "TRP" => "tryptophan",
        "MYRIC" => "myricetin",
        "QUERCE" => "quercetin",
        _ => return None,
    })
}

pub fn off_code(key: &str, registry: &NutrientRegistry) -> Option<String> {
    let code = match key {
        "energy-kj" | "energy-kcal" | "energy" => "energy",
        "proteins" => "protein",
        "carbohydrates" => "carbohydrate",
        "fiber" => "fibre",
        "soluble-fiber" => "soluble-fibre",
        "insoluble-fiber" => "insoluble-fibre",
        "vitamin-b1" => "thiamin",
        "vitamin-b2" | "vitamin-b2-riboflavin" => "riboflavin",
        "vitamin-b3-vitamin-pp-niacin" => "niacin",
        "vitamin-b5" => "pantothenic-acid",
        "vitamin-b6-pyridoxin" => "vitamin-b6",
        "vitamin-b12-cobalamin" => "vitamin-b12",
        "vitamin-pp" => "niacin",
        "vitamin-b9" | "folates" => "folate",
        other => other,
    };
    registry.contains(code).then(|| code.to_owned())
}

pub fn ignored_off_key(key: &str) -> bool {
    key.ends_with("_prepared")
        || key.starts_with("fruits-vegetables-")
        || matches!(
            key,
            "nova-group"
                | "nutrition-score-fr"
                | "cocoa"
                | "carbon-footprint"
                | "carbon-footprint-from-known-ingredients"
                | "collagen-meat-protein-ratio"
                | "ph"
        )
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use crate::model::{CanonicalUnit, NutrientRegistry};

    use super::*;

    #[test]
    fn fineli_mapping_covers_release_20() {
        let codes = [
            "ENERC", "FAT", "CHOAVL", "CHOCDF", "PROT", "ALC", "ASH", "WATER", "OA", "SUGOH",
            "SUGAR", "FRUS", "GALS", "GLUS", "LACS", "MALS", "SUCS", "STARCH", "FIBC", "FIBT",
            "FIBINS", "PSACNCS", "FOL", "NIAEQ", "NIA", "VITPYRID", "RIBF", "THIA", "VITA",
            "RETOL", "CAROTENS", "CARTB", "VITB12", "VITC", "VITD", "VITE", "VITK", "CA", "CR",
            "CU", "FD", "FE", "ID", "K", "MG", "MN", "MO", "NA", "NACL", "NT", "P", "SE", "ZN",
            "FAFRE", "FACIDCTG", "FAPU", "FAMCIS", "FASAT", "FATRN", "FAPUN3", "FAPUN6", "FAS18",
            "F16D0T", "F18D1T", "F18D2CN6", "F18D3N3", "F20D4N6", "F20D5N3", "F22D6N3", "CHOLE",
            "STERT", "TRP", "MYRIC", "QUERCE",
        ];
        assert_eq!(codes.len(), 74);
        let mapped = codes
            .iter()
            .map(|code| fineli_code(code).unwrap())
            .collect::<HashSet<_>>();
        assert_eq!(mapped.len(), 74);
    }

    #[test]
    fn off_aliases_only_resolve_known_canonical_codes() {
        let registry = NutrientRegistry::new(HashMap::from([
            ("protein".to_owned(), CanonicalUnit::Grams),
            ("pantothenic-acid".to_owned(), CanonicalUnit::Grams),
        ]));
        assert_eq!(off_code("proteins", &registry).as_deref(), Some("protein"));
        assert_eq!(
            off_code("vitamin-b5", &registry).as_deref(),
            Some("pantothenic-acid")
        );
        assert_eq!(off_code("made-up", &registry), None);
    }
}
