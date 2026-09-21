CREATE TABLE weight_entries (
    id uuid PRIMARY KEY,
    user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    weight_kg double precision NOT NULL
        CHECK (weight_kg > 0 AND weight_kg < 'Infinity'::double precision),
    measured_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX weight_entries_user_measured_idx
    ON weight_entries (user_id, measured_at DESC, id DESC);
