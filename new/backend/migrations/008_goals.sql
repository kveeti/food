CREATE TABLE goal_profiles (
    id uuid PRIMARY KEY,
    user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    starts_on date NOT NULL,
    daily_burn_kj double precision,
    food_adjustment_kj double precision,
    water_ml integer CHECK (water_ml BETWEEN 10 AND 100000),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (user_id, starts_on),
    CHECK ((daily_burn_kj IS NULL) = (food_adjustment_kj IS NULL)),
    CHECK (
        daily_burn_kj IS NULL
        OR (
            daily_burn_kj > 0
            AND daily_burn_kj < 'Infinity'::double precision
            AND abs(food_adjustment_kj) < 'Infinity'::double precision
            AND daily_burn_kj + food_adjustment_kj > 0
            AND daily_burn_kj + food_adjustment_kj < 'Infinity'::double precision
        )
    )
);

CREATE INDEX goal_profiles_user_starts_idx
    ON goal_profiles (user_id, starts_on DESC);

CREATE TABLE nutrient_goals (
    goal_profile_id uuid NOT NULL REFERENCES goal_profiles (id) ON DELETE CASCADE,
    nutrient_id smallint NOT NULL REFERENCES nutrients (id),
    value double precision NOT NULL
        CHECK (value > 0 AND value < 'Infinity'::double precision),
    PRIMARY KEY (goal_profile_id, nutrient_id)
);
