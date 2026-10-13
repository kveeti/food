CREATE TABLE nutrients (
    id smallint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    code text NOT NULL UNIQUE CHECK (code ~ '^[a-z0-9]+(-[a-z0-9]+)*$'),
    display_name text NOT NULL CHECK (btrim(display_name) <> ''),
    canonical_unit text NOT NULL CHECK (canonical_unit IN ('g', 'kJ')),
    display_unit text NOT NULL CHECK (display_unit IN ('g', 'mg', 'µg', 'kcal', 'kJ')),
    display_scale double precision NOT NULL
        CHECK (display_scale > 0 AND display_scale < 'Infinity'::double precision),
    category text NOT NULL CHECK (
        category IN ('energy', 'macro', 'fat', 'carbohydrate', 'vitamin', 'mineral', 'protein', 'other')
    ),
    display_order smallint NOT NULL UNIQUE,
    show_by_default boolean NOT NULL DEFAULT false,
    is_archived boolean NOT NULL DEFAULT false
);

CREATE TABLE foods (
    id uuid PRIMARY KEY,
    owner_user_id uuid REFERENCES users (id) ON DELETE CASCADE,
    source text CHECK (source IN ('fineli', 'open_food_facts')),
    source_id text,
    display_name text NOT NULL CHECK (btrim(display_name) <> ''),
    brand text CHECK (brand IS NULL OR btrim(brand) <> ''),
    basis_unit text NOT NULL CHECK (basis_unit IN ('g', 'ml')),
    source_data jsonb NOT NULL DEFAULT '{}'::jsonb
        CHECK (jsonb_typeof(source_data) = 'object'),
    is_archived boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (
        (owner_user_id IS NOT NULL AND source IS NULL AND source_id IS NULL)
        OR
        (owner_user_id IS NULL AND source IS NOT NULL AND source_id IS NOT NULL
            AND btrim(source_id) <> '')
    ),
    UNIQUE (source, source_id)
);

CREATE INDEX foods_owner_active_idx
    ON foods (owner_user_id, updated_at DESC)
    WHERE owner_user_id IS NOT NULL AND NOT is_archived;

CREATE TABLE food_names (
    food_id uuid NOT NULL REFERENCES foods (id) ON DELETE CASCADE,
    name text NOT NULL CHECK (btrim(name) <> ''),
    locale text CHECK (locale IS NULL OR btrim(locale) <> '')
);

CREATE UNIQUE INDEX food_names_food_name_locale_unique_idx
    ON food_names (food_id, lower(name), COALESCE(lower(locale), ''));

CREATE TABLE food_nutrients (
    food_id uuid NOT NULL REFERENCES foods (id) ON DELETE CASCADE,
    nutrient_id smallint NOT NULL REFERENCES nutrients (id),
    value double precision NOT NULL
        CHECK (value >= 0 AND value < 'Infinity'::double precision),
    PRIMARY KEY (food_id, nutrient_id)
);

CREATE TABLE off_sync_state (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    last_delta_end bigint,
    last_full_import_at timestamptz,
    updated_at timestamptz NOT NULL DEFAULT now()
);

INSERT INTO off_sync_state (singleton) VALUES (true);

INSERT INTO nutrients (
    code,
    display_name,
    canonical_unit,
    display_unit,
    display_scale,
    category,
    display_order,
    show_by_default,
    is_archived
)
VALUES
    ('energy', 'Energy', 'kJ', 'kcal', 0.2390057361376673, 'energy', 0, true, false),
    ('fat', 'Fat', 'g', 'g', 1.0, 'macro', 1, true, false),
    ('saturated-fat', 'Saturated fat', 'g', 'g', 1.0, 'fat', 2, false, false),
    ('carbohydrate', 'Carbs', 'g', 'g', 1.0, 'macro', 3, true, false),
    ('sugars', 'Sugars', 'g', 'g', 1.0, 'carbohydrate', 4, false, false),
    ('fibre', 'Fibre', 'g', 'g', 1.0, 'carbohydrate', 5, true, false),
    ('protein', 'Protein', 'g', 'g', 1.0, 'macro', 6, true, false),
    ('salt', 'Salt', 'g', 'g', 1.0, 'mineral', 7, false, false),
    ('sodium', 'Sodium', 'g', 'mg', 1000.0, 'mineral', 8, false, false),
    ('water', 'Water', 'g', 'g', 1.0, 'other', 9, false, false),
    ('alcohol', 'Alcohol', 'g', 'g', 1.0, 'other', 10, false, false),
    ('ash', 'Ash', 'g', 'g', 1.0, 'other', 11, false, false),
    ('organic-acids', 'Organic acids', 'g', 'g', 1.0, 'carbohydrate', 12, false, false),
    ('polyols', 'Polyols', 'g', 'g', 1.0, 'carbohydrate', 13, false, false),
    ('starch', 'Starch', 'g', 'g', 1.0, 'carbohydrate', 14, false, false),
    ('fructose', 'Fructose', 'g', 'g', 1.0, 'carbohydrate', 15, false, false),
    ('galactose', 'Galactose', 'g', 'g', 1.0, 'carbohydrate', 16, false, false),
    ('glucose', 'Glucose', 'g', 'g', 1.0, 'carbohydrate', 17, false, false),
    ('lactose', 'Lactose', 'g', 'g', 1.0, 'carbohydrate', 18, false, false),
    ('maltose', 'Maltose', 'g', 'g', 1.0, 'carbohydrate', 19, false, false),
    ('sucrose', 'Sucrose', 'g', 'g', 1.0, 'carbohydrate', 20, false, false),
    ('carbohydrate-by-difference', 'Carbohydrate by difference', 'g', 'g', 1.0, 'carbohydrate', 21, false, false),
    ('dietary-fibre', 'Dietary fibre', 'g', 'g', 1.0, 'carbohydrate', 22, false, false),
    ('insoluble-fibre', 'Insoluble fibre', 'g', 'g', 1.0, 'carbohydrate', 23, false, false),
    ('soluble-fibre', 'Soluble fibre', 'g', 'g', 1.0, 'carbohydrate', 24, false, false),
    ('soluble-non-cellulosic-polysaccharides', 'Soluble non-cellulosic polysaccharides', 'g', 'g', 1.0, 'carbohydrate', 25, false, false),
    ('added-sugars', 'Added sugars', 'g', 'g', 1.0, 'carbohydrate', 26, false, false),
    ('maltodextrins', 'Maltodextrins', 'g', 'g', 1.0, 'carbohydrate', 27, false, false),
    ('psicose', 'Psicose', 'g', 'g', 1.0, 'carbohydrate', 28, false, false),
    ('erythritol', 'Erythritol', 'g', 'g', 1.0, 'carbohydrate', 29, false, false),
    ('isomalt', 'Isomalt', 'g', 'g', 1.0, 'carbohydrate', 30, false, false),
    ('maltitol', 'Maltitol', 'g', 'g', 1.0, 'carbohydrate', 31, false, false),
    ('sorbitol', 'Sorbitol', 'g', 'g', 1.0, 'carbohydrate', 32, false, false),
    ('polydextrose', 'Polydextrose', 'g', 'g', 1.0, 'carbohydrate', 33, false, false),
    ('beta-glucan', 'Beta-glucan', 'g', 'g', 1.0, 'carbohydrate', 34, false, false),
    ('energy-from-fat', 'Energy from fat', 'kJ', 'kcal', 0.2390057361376673, 'energy', 35, false, false),
    ('fatty-acids', 'Fatty acids', 'g', 'g', 1.0, 'fat', 36, false, false),
    ('fatty-acids-tag-equivalent', 'Fatty acids as triacylglycerol equivalents', 'g', 'g', 1.0, 'fat', 37, false, false),
    ('unsaturated-fat', 'Unsaturated fat', 'g', 'g', 1.0, 'fat', 38, false, false),
    ('monounsaturated-fat', 'Monounsaturated fat', 'g', 'g', 1.0, 'fat', 39, false, false),
    ('polyunsaturated-fat', 'Polyunsaturated fat', 'g', 'g', 1.0, 'fat', 40, false, false),
    ('trans-fat', 'Trans fat', 'g', 'g', 1.0, 'fat', 41, false, false),
    ('omega-3-fat', 'Omega-3 fat', 'g', 'g', 1.0, 'fat', 42, false, false),
    ('omega-6-fat', 'Omega-6 fat', 'g', 'g', 1.0, 'fat', 43, false, false),
    ('omega-9-fat', 'Omega-9 fat', 'g', 'g', 1.0, 'fat', 44, false, false),
    ('butyric-acid', 'Butyric acid', 'g', 'g', 1.0, 'fat', 45, false, false),
    ('caproic-acid', 'Caproic acid', 'g', 'g', 1.0, 'fat', 46, false, false),
    ('caprylic-acid', 'Caprylic acid', 'g', 'g', 1.0, 'fat', 47, false, false),
    ('capric-acid', 'Capric acid', 'g', 'g', 1.0, 'fat', 48, false, false),
    ('lauric-acid', 'Lauric acid', 'g', 'g', 1.0, 'fat', 49, false, false),
    ('myristic-acid', 'Myristic acid', 'g', 'g', 1.0, 'fat', 50, false, false),
    ('palmitic-acid', 'Palmitic acid', 'g', 'g', 1.0, 'fat', 51, false, false),
    ('stearic-acid', 'Stearic acid', 'g', 'g', 1.0, 'fat', 52, false, false),
    ('arachidic-acid', 'Arachidic acid', 'g', 'g', 1.0, 'fat', 53, false, false),
    ('behenic-acid', 'Behenic acid', 'g', 'g', 1.0, 'fat', 54, false, false),
    ('lignoceric-acid', 'Lignoceric acid', 'g', 'g', 1.0, 'fat', 55, false, false),
    ('cerotic-acid', 'Cerotic acid', 'g', 'g', 1.0, 'fat', 56, false, false),
    ('montanic-acid', 'Montanic acid', 'g', 'g', 1.0, 'fat', 57, false, false),
    ('melissic-acid', 'Melissic acid', 'g', 'g', 1.0, 'fat', 58, false, false),
    ('alpha-linolenic-acid', 'Alpha-linolenic acid', 'g', 'g', 1.0, 'fat', 59, false, false),
    ('eicosapentaenoic-acid', 'Eicosapentaenoic acid (EPA)', 'g', 'g', 1.0, 'fat', 60, false, false),
    ('docosahexaenoic-acid', 'Docosahexaenoic acid (DHA)', 'g', 'g', 1.0, 'fat', 61, false, false),
    ('linoleic-acid', 'Linoleic acid', 'g', 'g', 1.0, 'fat', 62, false, false),
    ('arachidonic-acid', 'Arachidonic acid', 'g', 'g', 1.0, 'fat', 63, false, false),
    ('gamma-linolenic-acid', 'Gamma-linolenic acid', 'g', 'g', 1.0, 'fat', 64, false, false),
    ('dihomo-gamma-linolenic-acid', 'Dihomo-gamma-linolenic acid', 'g', 'g', 1.0, 'fat', 65, false, false),
    ('oleic-acid', 'Oleic acid', 'g', 'g', 1.0, 'fat', 66, false, false),
    ('elaidic-acid', 'Elaidic acid', 'g', 'g', 1.0, 'fat', 67, false, false),
    ('gondoic-acid', 'Gondoic acid', 'g', 'g', 1.0, 'fat', 68, false, false),
    ('mead-acid', 'Mead acid', 'g', 'g', 1.0, 'fat', 69, false, false),
    ('erucic-acid', 'Erucic acid', 'g', 'g', 1.0, 'fat', 70, false, false),
    ('nervonic-acid', 'Nervonic acid', 'g', 'g', 1.0, 'fat', 71, false, false),
    ('fatty-acid-18-1', 'Fatty acid 18:1', 'g', 'g', 1.0, 'fat', 72, false, false),
    ('cholesterol', 'Cholesterol', 'g', 'mg', 1000.0, 'fat', 73, false, false),
    ('sterols', 'Sterols', 'g', 'mg', 1000.0, 'fat', 74, false, false),
    ('vitamin-a', 'Vitamin A', 'g', 'µg', 1000000.0, 'vitamin', 75, false, false),
    ('retinol', 'Retinol', 'g', 'µg', 1000000.0, 'vitamin', 76, false, false),
    ('carotenoids', 'Carotenoids', 'g', 'µg', 1000000.0, 'vitamin', 77, false, false),
    ('beta-carotene', 'Beta-carotene', 'g', 'µg', 1000000.0, 'vitamin', 78, false, false),
    ('vitamin-b6', 'Vitamin B6', 'g', 'mg', 1000.0, 'vitamin', 79, false, false),
    ('vitamin-b12', 'Vitamin B12', 'g', 'µg', 1000000.0, 'vitamin', 80, false, false),
    ('thiamin', 'Thiamin', 'g', 'mg', 1000.0, 'vitamin', 81, false, false),
    ('riboflavin', 'Riboflavin', 'g', 'mg', 1000.0, 'vitamin', 82, false, false),
    ('niacin', 'Niacin', 'g', 'mg', 1000.0, 'vitamin', 83, false, false),
    ('niacin-equivalents', 'Niacin equivalents', 'g', 'mg', 1000.0, 'vitamin', 84, false, false),
    ('folate', 'Folate', 'g', 'µg', 1000000.0, 'vitamin', 85, false, false),
    ('pantothenic-acid', 'Pantothenic acid', 'g', 'mg', 1000.0, 'vitamin', 86, false, false),
    ('biotin', 'Biotin', 'g', 'µg', 1000000.0, 'vitamin', 87, false, false),
    ('vitamin-c', 'Vitamin C', 'g', 'mg', 1000.0, 'vitamin', 88, false, false),
    ('vitamin-d', 'Vitamin D', 'g', 'µg', 1000000.0, 'vitamin', 89, false, false),
    ('vitamin-e', 'Vitamin E', 'g', 'mg', 1000.0, 'vitamin', 90, false, false),
    ('vitamin-k', 'Vitamin K', 'g', 'µg', 1000000.0, 'vitamin', 91, false, false),
    ('phylloquinone', 'Phylloquinone', 'g', 'µg', 1000000.0, 'vitamin', 92, false, false),
    ('calcium', 'Calcium', 'g', 'mg', 1000.0, 'mineral', 93, false, false),
    ('chloride', 'Chloride', 'g', 'mg', 1000.0, 'mineral', 94, false, false),
    ('chromium', 'Chromium', 'g', 'µg', 1000000.0, 'mineral', 95, false, false),
    ('copper', 'Copper', 'g', 'mg', 1000.0, 'mineral', 96, false, false),
    ('fluoride', 'Fluoride', 'g', 'mg', 1000.0, 'mineral', 97, false, false),
    ('iron', 'Iron', 'g', 'mg', 1000.0, 'mineral', 98, false, false),
    ('iodine', 'Iodine', 'g', 'µg', 1000000.0, 'mineral', 99, false, false),
    ('potassium', 'Potassium', 'g', 'mg', 1000.0, 'mineral', 100, false, false),
    ('magnesium', 'Magnesium', 'g', 'mg', 1000.0, 'mineral', 101, false, false),
    ('manganese', 'Manganese', 'g', 'mg', 1000.0, 'mineral', 102, false, false),
    ('molybdenum', 'Molybdenum', 'g', 'µg', 1000000.0, 'mineral', 103, false, false),
    ('phosphorus', 'Phosphorus', 'g', 'mg', 1000.0, 'mineral', 104, false, false),
    ('selenium', 'Selenium', 'g', 'µg', 1000000.0, 'mineral', 105, false, false),
    ('zinc', 'Zinc', 'g', 'mg', 1000.0, 'mineral', 106, false, false),
    ('silica', 'Silica', 'g', 'mg', 1000.0, 'mineral', 107, false, false),
    ('bicarbonate', 'Bicarbonate', 'g', 'mg', 1000.0, 'mineral', 108, false, false),
    ('sulphate', 'Sulphate', 'g', 'mg', 1000.0, 'mineral', 109, false, false),
    ('nitrate', 'Nitrate', 'g', 'mg', 1000.0, 'mineral', 110, false, false),
    ('nitrogen', 'Nitrogen', 'g', 'g', 1.0, 'other', 111, false, false),
    ('tryptophan', 'Tryptophan', 'g', 'mg', 1000.0, 'other', 112, false, false),
    ('casein', 'Casein', 'g', 'g', 1.0, 'protein', 113, false, false),
    ('serum-proteins', 'Serum proteins', 'g', 'g', 1.0, 'protein', 114, false, false),
    ('nucleotides', 'Nucleotides', 'g', 'mg', 1000.0, 'other', 115, false, false),
    ('added-salt', 'Added salt', 'g', 'g', 1.0, 'other', 116, false, false),
    ('myricetin', 'Myricetin', 'g', 'mg', 1000.0, 'other', 117, false, false),
    ('quercetin', 'Quercetin', 'g', 'mg', 1000.0, 'other', 118, false, false),
    ('caffeine', 'Caffeine', 'g', 'mg', 1000.0, 'other', 119, false, false),
    ('taurine', 'Taurine', 'g', 'mg', 1000.0, 'other', 120, false, false),
    ('methylsulfonylmethane', 'Methylsulfonylmethane', 'g', 'mg', 1000.0, 'other', 121, false, false),
    ('hydroxymethylbutyrate', 'Hydroxymethylbutyrate', 'g', 'mg', 1000.0, 'other', 122, false, false),
    ('choline', 'Choline', 'g', 'mg', 1000.0, 'other', 123, false, false),
    ('inositol', 'Inositol', 'g', 'mg', 1000.0, 'other', 124, false, false),
    ('carnitine', 'Carnitine', 'g', 'mg', 1000.0, 'other', 125, false, false);
