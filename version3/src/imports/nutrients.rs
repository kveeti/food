use super::model::NutrientDef;

struct Nutrient {
    code: &'static str,
    name: &'static str,
    category: &'static str,
}

const NUTRIENTS: &[Nutrient] = &[
    Nutrient {
        code: "energy",
        name: "Energy",
        category: "energy",
    },
    Nutrient {
        code: "fat",
        name: "Fat",
        category: "macro",
    },
    Nutrient {
        code: "saturated-fat",
        name: "Saturated fat",
        category: "fat",
    },
    Nutrient {
        code: "carbohydrate",
        name: "Carbs",
        category: "macro",
    },
    Nutrient {
        code: "sugars",
        name: "Sugars",
        category: "carbohydrate",
    },
    Nutrient {
        code: "fibre",
        name: "Fibre",
        category: "carbohydrate",
    },
    Nutrient {
        code: "protein",
        name: "Protein",
        category: "macro",
    },
    Nutrient {
        code: "salt",
        name: "Salt",
        category: "mineral",
    },
    Nutrient {
        code: "sodium",
        name: "Sodium",
        category: "mineral",
    },
    Nutrient {
        code: "water",
        name: "Water",
        category: "other",
    },
    Nutrient {
        code: "alcohol",
        name: "Alcohol",
        category: "other",
    },
    Nutrient {
        code: "ash",
        name: "Ash",
        category: "other",
    },
    Nutrient {
        code: "organic-acids",
        name: "Organic acids",
        category: "carbohydrate",
    },
    Nutrient {
        code: "polyols",
        name: "Polyols",
        category: "carbohydrate",
    },
    Nutrient {
        code: "starch",
        name: "Starch",
        category: "carbohydrate",
    },
    Nutrient {
        code: "fructose",
        name: "Fructose",
        category: "carbohydrate",
    },
    Nutrient {
        code: "galactose",
        name: "Galactose",
        category: "carbohydrate",
    },
    Nutrient {
        code: "glucose",
        name: "Glucose",
        category: "carbohydrate",
    },
    Nutrient {
        code: "lactose",
        name: "Lactose",
        category: "carbohydrate",
    },
    Nutrient {
        code: "maltose",
        name: "Maltose",
        category: "carbohydrate",
    },
    Nutrient {
        code: "sucrose",
        name: "Sucrose",
        category: "carbohydrate",
    },
    Nutrient {
        code: "carbohydrate-by-difference",
        name: "Carbohydrate by difference",
        category: "carbohydrate",
    },
    Nutrient {
        code: "dietary-fibre",
        name: "Dietary fibre",
        category: "carbohydrate",
    },
    Nutrient {
        code: "insoluble-fibre",
        name: "Insoluble fibre",
        category: "carbohydrate",
    },
    Nutrient {
        code: "soluble-fibre",
        name: "Soluble fibre",
        category: "carbohydrate",
    },
    Nutrient {
        code: "soluble-non-cellulosic-polysaccharides",
        name: "Soluble non-cellulosic polysaccharides",
        category: "carbohydrate",
    },
    Nutrient {
        code: "added-sugars",
        name: "Added sugars",
        category: "carbohydrate",
    },
    Nutrient {
        code: "maltodextrins",
        name: "Maltodextrins",
        category: "carbohydrate",
    },
    Nutrient {
        code: "psicose",
        name: "Psicose",
        category: "carbohydrate",
    },
    Nutrient {
        code: "erythritol",
        name: "Erythritol",
        category: "carbohydrate",
    },
    Nutrient {
        code: "isomalt",
        name: "Isomalt",
        category: "carbohydrate",
    },
    Nutrient {
        code: "maltitol",
        name: "Maltitol",
        category: "carbohydrate",
    },
    Nutrient {
        code: "sorbitol",
        name: "Sorbitol",
        category: "carbohydrate",
    },
    Nutrient {
        code: "polydextrose",
        name: "Polydextrose",
        category: "carbohydrate",
    },
    Nutrient {
        code: "beta-glucan",
        name: "Beta-glucan",
        category: "carbohydrate",
    },
    Nutrient {
        code: "energy-from-fat",
        name: "Energy from fat",
        category: "energy",
    },
    Nutrient {
        code: "fatty-acids",
        name: "Fatty acids",
        category: "fat",
    },
    Nutrient {
        code: "fatty-acids-tag-equivalent",
        name: "Fatty acids as triacylglycerol equivalents",
        category: "fat",
    },
    Nutrient {
        code: "unsaturated-fat",
        name: "Unsaturated fat",
        category: "fat",
    },
    Nutrient {
        code: "monounsaturated-fat",
        name: "Monounsaturated fat",
        category: "fat",
    },
    Nutrient {
        code: "polyunsaturated-fat",
        name: "Polyunsaturated fat",
        category: "fat",
    },
    Nutrient {
        code: "trans-fat",
        name: "Trans fat",
        category: "fat",
    },
    Nutrient {
        code: "omega-3-fat",
        name: "Omega-3 fat",
        category: "fat",
    },
    Nutrient {
        code: "omega-6-fat",
        name: "Omega-6 fat",
        category: "fat",
    },
    Nutrient {
        code: "omega-9-fat",
        name: "Omega-9 fat",
        category: "fat",
    },
    Nutrient {
        code: "butyric-acid",
        name: "Butyric acid",
        category: "fat",
    },
    Nutrient {
        code: "caproic-acid",
        name: "Caproic acid",
        category: "fat",
    },
    Nutrient {
        code: "caprylic-acid",
        name: "Caprylic acid",
        category: "fat",
    },
    Nutrient {
        code: "capric-acid",
        name: "Capric acid",
        category: "fat",
    },
    Nutrient {
        code: "lauric-acid",
        name: "Lauric acid",
        category: "fat",
    },
    Nutrient {
        code: "myristic-acid",
        name: "Myristic acid",
        category: "fat",
    },
    Nutrient {
        code: "palmitic-acid",
        name: "Palmitic acid",
        category: "fat",
    },
    Nutrient {
        code: "stearic-acid",
        name: "Stearic acid",
        category: "fat",
    },
    Nutrient {
        code: "arachidic-acid",
        name: "Arachidic acid",
        category: "fat",
    },
    Nutrient {
        code: "behenic-acid",
        name: "Behenic acid",
        category: "fat",
    },
    Nutrient {
        code: "lignoceric-acid",
        name: "Lignoceric acid",
        category: "fat",
    },
    Nutrient {
        code: "cerotic-acid",
        name: "Cerotic acid",
        category: "fat",
    },
    Nutrient {
        code: "montanic-acid",
        name: "Montanic acid",
        category: "fat",
    },
    Nutrient {
        code: "melissic-acid",
        name: "Melissic acid",
        category: "fat",
    },
    Nutrient {
        code: "alpha-linolenic-acid",
        name: "Alpha-linolenic acid",
        category: "fat",
    },
    Nutrient {
        code: "eicosapentaenoic-acid",
        name: "Eicosapentaenoic acid (EPA)",
        category: "fat",
    },
    Nutrient {
        code: "docosahexaenoic-acid",
        name: "Docosahexaenoic acid (DHA)",
        category: "fat",
    },
    Nutrient {
        code: "linoleic-acid",
        name: "Linoleic acid",
        category: "fat",
    },
    Nutrient {
        code: "arachidonic-acid",
        name: "Arachidonic acid",
        category: "fat",
    },
    Nutrient {
        code: "gamma-linolenic-acid",
        name: "Gamma-linolenic acid",
        category: "fat",
    },
    Nutrient {
        code: "dihomo-gamma-linolenic-acid",
        name: "Dihomo-gamma-linolenic acid",
        category: "fat",
    },
    Nutrient {
        code: "oleic-acid",
        name: "Oleic acid",
        category: "fat",
    },
    Nutrient {
        code: "elaidic-acid",
        name: "Elaidic acid",
        category: "fat",
    },
    Nutrient {
        code: "gondoic-acid",
        name: "Gondoic acid",
        category: "fat",
    },
    Nutrient {
        code: "mead-acid",
        name: "Mead acid",
        category: "fat",
    },
    Nutrient {
        code: "erucic-acid",
        name: "Erucic acid",
        category: "fat",
    },
    Nutrient {
        code: "nervonic-acid",
        name: "Nervonic acid",
        category: "fat",
    },
    Nutrient {
        code: "fatty-acid-18-1",
        name: "Fatty acid 18:1",
        category: "fat",
    },
    Nutrient {
        code: "cholesterol",
        name: "Cholesterol",
        category: "fat",
    },
    Nutrient {
        code: "sterols",
        name: "Sterols",
        category: "fat",
    },
    Nutrient {
        code: "vitamin-a",
        name: "Vitamin A",
        category: "vitamin",
    },
    Nutrient {
        code: "retinol",
        name: "Retinol",
        category: "vitamin",
    },
    Nutrient {
        code: "carotenoids",
        name: "Carotenoids",
        category: "vitamin",
    },
    Nutrient {
        code: "beta-carotene",
        name: "Beta-carotene",
        category: "vitamin",
    },
    Nutrient {
        code: "vitamin-b6",
        name: "Vitamin B6",
        category: "vitamin",
    },
    Nutrient {
        code: "vitamin-b12",
        name: "Vitamin B12",
        category: "vitamin",
    },
    Nutrient {
        code: "thiamin",
        name: "Thiamin",
        category: "vitamin",
    },
    Nutrient {
        code: "riboflavin",
        name: "Riboflavin",
        category: "vitamin",
    },
    Nutrient {
        code: "niacin",
        name: "Niacin",
        category: "vitamin",
    },
    Nutrient {
        code: "niacin-equivalents",
        name: "Niacin equivalents",
        category: "vitamin",
    },
    Nutrient {
        code: "folate",
        name: "Folate",
        category: "vitamin",
    },
    Nutrient {
        code: "pantothenic-acid",
        name: "Pantothenic acid",
        category: "vitamin",
    },
    Nutrient {
        code: "biotin",
        name: "Biotin",
        category: "vitamin",
    },
    Nutrient {
        code: "vitamin-c",
        name: "Vitamin C",
        category: "vitamin",
    },
    Nutrient {
        code: "vitamin-d",
        name: "Vitamin D",
        category: "vitamin",
    },
    Nutrient {
        code: "vitamin-e",
        name: "Vitamin E",
        category: "vitamin",
    },
    Nutrient {
        code: "vitamin-k",
        name: "Vitamin K",
        category: "vitamin",
    },
    Nutrient {
        code: "phylloquinone",
        name: "Phylloquinone",
        category: "vitamin",
    },
    Nutrient {
        code: "calcium",
        name: "Calcium",
        category: "mineral",
    },
    Nutrient {
        code: "chloride",
        name: "Chloride",
        category: "mineral",
    },
    Nutrient {
        code: "chromium",
        name: "Chromium",
        category: "mineral",
    },
    Nutrient {
        code: "copper",
        name: "Copper",
        category: "mineral",
    },
    Nutrient {
        code: "fluoride",
        name: "Fluoride",
        category: "mineral",
    },
    Nutrient {
        code: "iron",
        name: "Iron",
        category: "mineral",
    },
    Nutrient {
        code: "iodine",
        name: "Iodine",
        category: "mineral",
    },
    Nutrient {
        code: "potassium",
        name: "Potassium",
        category: "mineral",
    },
    Nutrient {
        code: "magnesium",
        name: "Magnesium",
        category: "mineral",
    },
    Nutrient {
        code: "manganese",
        name: "Manganese",
        category: "mineral",
    },
    Nutrient {
        code: "molybdenum",
        name: "Molybdenum",
        category: "mineral",
    },
    Nutrient {
        code: "phosphorus",
        name: "Phosphorus",
        category: "mineral",
    },
    Nutrient {
        code: "selenium",
        name: "Selenium",
        category: "mineral",
    },
    Nutrient {
        code: "zinc",
        name: "Zinc",
        category: "mineral",
    },
    Nutrient {
        code: "silica",
        name: "Silica",
        category: "mineral",
    },
    Nutrient {
        code: "bicarbonate",
        name: "Bicarbonate",
        category: "mineral",
    },
    Nutrient {
        code: "sulphate",
        name: "Sulphate",
        category: "mineral",
    },
    Nutrient {
        code: "nitrate",
        name: "Nitrate",
        category: "mineral",
    },
    Nutrient {
        code: "nitrogen",
        name: "Nitrogen",
        category: "other",
    },
    Nutrient {
        code: "tryptophan",
        name: "Tryptophan",
        category: "other",
    },
    Nutrient {
        code: "casein",
        name: "Casein",
        category: "protein",
    },
    Nutrient {
        code: "serum-proteins",
        name: "Serum proteins",
        category: "protein",
    },
    Nutrient {
        code: "nucleotides",
        name: "Nucleotides",
        category: "other",
    },
    Nutrient {
        code: "added-salt",
        name: "Added salt",
        category: "other",
    },
    Nutrient {
        code: "myricetin",
        name: "Myricetin",
        category: "other",
    },
    Nutrient {
        code: "quercetin",
        name: "Quercetin",
        category: "other",
    },
    Nutrient {
        code: "caffeine",
        name: "Caffeine",
        category: "other",
    },
    Nutrient {
        code: "taurine",
        name: "Taurine",
        category: "other",
    },
    Nutrient {
        code: "methylsulfonylmethane",
        name: "Methylsulfonylmethane",
        category: "other",
    },
    Nutrient {
        code: "hydroxymethylbutyrate",
        name: "Hydroxymethylbutyrate",
        category: "other",
    },
    Nutrient {
        code: "choline",
        name: "Choline",
        category: "other",
    },
    Nutrient {
        code: "inositol",
        name: "Inositol",
        category: "other",
    },
    Nutrient {
        code: "carnitine",
        name: "Carnitine",
        category: "other",
    },
];

pub fn definition(code: &str) -> Option<NutrientDef> {
    let (display_order, nutrient) = NUTRIENTS
        .iter()
        .enumerate()
        .find(|(_, nutrient)| nutrient.code == code)?;
    Some(NutrientDef {
        code: nutrient.code.to_owned(),
        display_name: nutrient.name.to_owned(),
        unit: if nutrient.category == "energy" {
            "kJ"
        } else {
            "g"
        }
        .to_owned(),
        category: nutrient.category.to_owned(),
        display_order: display_order as i32,
    })
}

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

pub fn off_code(key: &str) -> Option<&str> {
    let code = match key {
        "energy-kj" | "energy-kcal" | "energy" => "energy",
        "proteins" => "protein",
        "carbohydrates" => "carbohydrate",
        "fiber" => "fibre",
        "soluble-fiber" => "soluble-fibre",
        "insoluble-fiber" => "insoluble-fibre",
        "vitamin-b1" => "thiamin",
        "vitamin-b2" => "riboflavin",
        "vitamin-pp" => "niacin",
        "vitamin-b9" | "folates" => "folate",
        other => other,
    };
    NUTRIENTS
        .iter()
        .any(|nutrient| nutrient.code == code)
        .then_some(code)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn owns_canonical_names() {
        assert_eq!(definition("protein").unwrap().display_name, "Protein");
        assert_eq!(definition("carbohydrate").unwrap().display_name, "Carbs");
        assert_eq!(definition("fat").unwrap().display_name, "Fat");
        assert_eq!(fineli_code("PROT"), Some("protein"));
        assert_eq!(off_code("proteins"), Some("protein"));
        assert_eq!(off_code("insoluble-fiber"), Some("insoluble-fibre"));
    }

    #[test]
    fn canonical_codes_are_unique() {
        let codes: HashSet<&str> = NUTRIENTS.iter().map(|nutrient| nutrient.code).collect();
        assert_eq!(codes.len(), NUTRIENTS.len());
    }
}
