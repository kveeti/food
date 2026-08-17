CREATE TABLE IF NOT EXISTS water_log (
    id          UUID PRIMARY KEY DEFAULT uuidv7(),
    amount_ml   INTEGER NOT NULL CHECK (amount_ml > 0),
    consumed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
