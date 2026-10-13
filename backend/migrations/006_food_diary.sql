CREATE TABLE meals (
    id uuid PRIMARY KEY,
    user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    name text CHECK (name IS NULL OR btrim(name) <> ''),
    started_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX meals_user_started_idx
    ON meals (user_id, started_at DESC);

CREATE TABLE food_entries (
    id uuid PRIMARY KEY,
    user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    meal_id uuid REFERENCES meals (id) ON DELETE SET NULL,
    food_id uuid REFERENCES foods (id) ON DELETE SET NULL,
    amount double precision NOT NULL
        CHECK (amount > 0 AND amount <= 100000 AND amount < 'Infinity'::double precision),
    unit text NOT NULL CHECK (unit IN ('g', 'ml')),
    eaten_at timestamptz NOT NULL,
    food_name text NOT NULL CHECK (btrim(food_name) <> ''),
    food_brand text CHECK (food_brand IS NULL OR btrim(food_brand) <> ''),
    food_source text CHECK (food_source IN ('fineli', 'open_food_facts')),
    food_source_id text,
    created_at timestamptz NOT NULL DEFAULT now(),
    CHECK (
        (food_source IS NULL AND food_source_id IS NULL)
        OR
        (food_source IS NOT NULL AND food_source_id IS NOT NULL AND btrim(food_source_id) <> '')
    )
);

CREATE INDEX food_entries_user_eaten_idx
    ON food_entries (user_id, eaten_at DESC);
CREATE INDEX food_entries_food_idx ON food_entries (food_id);
CREATE INDEX food_entries_meal_idx ON food_entries (meal_id);

CREATE TABLE food_entry_nutrients (
    food_entry_id uuid NOT NULL REFERENCES food_entries (id) ON DELETE CASCADE,
    nutrient_id smallint NOT NULL REFERENCES nutrients (id),
    basis_value double precision NOT NULL
        CHECK (basis_value >= 0 AND basis_value < 'Infinity'::double precision),
    PRIMARY KEY (food_entry_id, nutrient_id)
);

CREATE INDEX food_entry_nutrients_nutrient_idx
    ON food_entry_nutrients (nutrient_id);
