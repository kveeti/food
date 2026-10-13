CREATE TABLE water_entries (
    id uuid PRIMARY KEY,
    user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    amount_ml integer NOT NULL CHECK (amount_ml BETWEEN 1 AND 10000),
    consumed_at timestamptz NOT NULL DEFAULT now(),
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX water_entries_user_consumed_idx
    ON water_entries (user_id, consumed_at DESC);
