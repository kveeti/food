CREATE TABLE IF NOT EXISTS users (
    id         UUID PRIMARY KEY DEFAULT uuidv7(),
    issuer     TEXT NOT NULL,
    subject    TEXT NOT NULL,
    email      TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (issuer, subject)
);

ALTER TABLE users ADD COLUMN IF NOT EXISTS timezone TEXT;

CREATE TABLE IF NOT EXISTS sessions (
    token_hash BYTEA PRIMARY KEY,
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    is_admin   BOOLEAN NOT NULL DEFAULT false,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
ALTER TABLE sessions
    ADD COLUMN IF NOT EXISTS is_admin BOOLEAN NOT NULL DEFAULT false;
CREATE INDEX IF NOT EXISTS sessions_user_id_idx ON sessions(user_id);

CREATE TABLE IF NOT EXISTS import_settings (
    singleton            BOOLEAN PRIMARY KEY DEFAULT true CHECK (singleton),
    off_full_url         TEXT NOT NULL,
    off_delta_index_url  TEXT NOT NULL,
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);
INSERT INTO import_settings (off_full_url, off_delta_index_url)
VALUES (
    'https://static.openfoodfacts.org/data/openfoodfacts-products.jsonl.gz',
    'https://static.openfoodfacts.org/data/delta/index.txt'
)
ON CONFLICT (singleton) DO NOTHING;

CREATE TABLE IF NOT EXISTS import_jobs (
    id             UUID PRIMARY KEY DEFAULT uuidv7(),
    kind           TEXT NOT NULL CHECK (kind IN (
                       'fineli_upload', 'off_full_upload', 'off_delta_upload',
                       'off_full_sync', 'off_delta_sync'
                   )),
    status         TEXT NOT NULL DEFAULT 'queued' CHECK (status IN (
                       'queued', 'running', 'succeeded', 'failed'
                   )),
    file_path      TEXT,
    original_name  TEXT,
    sha256         TEXT,
    requested_by   UUID REFERENCES users(id) ON DELETE SET NULL,
    delta_start    BIGINT,
    delta_end      BIGINT,
    details        TEXT,
    error          TEXT,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    started_at     TIMESTAMPTZ,
    finished_at    TIMESTAMPTZ
);
ALTER TABLE import_jobs
    ADD COLUMN IF NOT EXISTS sha256 TEXT;
CREATE INDEX IF NOT EXISTS import_jobs_status_created_idx
    ON import_jobs(status, created_at);
CREATE UNIQUE INDEX IF NOT EXISTS import_jobs_active_kind_idx
    ON import_jobs(kind) WHERE status IN ('queued', 'running')
      AND kind IN ('off_full_sync', 'off_delta_sync');

CREATE TABLE IF NOT EXISTS off_sync_state (
    singleton       BOOLEAN PRIMARY KEY DEFAULT true CHECK (singleton),
    last_delta_end  BIGINT,
    last_full_at    TIMESTAMPTZ,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
INSERT INTO off_sync_state DEFAULT VALUES
ON CONFLICT (singleton) DO NOTHING;

CREATE TABLE IF NOT EXISTS water_log (
    id          UUID PRIMARY KEY DEFAULT uuidv7(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    amount_ml   INTEGER NOT NULL CHECK (amount_ml > 0),
    consumed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS water_log_user_consumed_idx
    ON water_log(user_id, consumed_at DESC);

CREATE TABLE IF NOT EXISTS nutrients (
    id            UUID PRIMARY KEY DEFAULT uuidv7(),
    code          TEXT NOT NULL UNIQUE,
    display_name  TEXT NOT NULL,
    unit          TEXT NOT NULL CHECK (unit IN ('g', 'kJ')),
    category      TEXT NOT NULL,
    display_order INTEGER NOT NULL DEFAULT 0,
    is_archived   BOOLEAN NOT NULL DEFAULT false,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE nutrients
    ADD COLUMN IF NOT EXISTS is_archived BOOLEAN NOT NULL DEFAULT false;

CREATE TABLE IF NOT EXISTS goal_profiles (
    id                       UUID PRIMARY KEY DEFAULT uuidv7(),
    user_id                  UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    effective_from           DATE NOT NULL,
    daily_burn_kcal          INTEGER CHECK (daily_burn_kcal > 0),
    food_adjustment_kcal     INTEGER NOT NULL DEFAULT 0,
    water_goal_ml            INTEGER CHECK (water_goal_ml > 0),
    created_at               TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at               TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, effective_from),
    CHECK (daily_burn_kcal IS NOT NULL OR food_adjustment_kcal = 0),
    CHECK (daily_burn_kcal IS NULL OR daily_burn_kcal + food_adjustment_kcal > 0)
);
CREATE INDEX IF NOT EXISTS goal_profiles_user_date_idx
    ON goal_profiles(user_id, effective_from DESC);

CREATE TABLE IF NOT EXISTS nutrient_goals (
    goal_profile_id UUID NOT NULL REFERENCES goal_profiles(id) ON DELETE CASCADE,
    nutrient_id     UUID NOT NULL REFERENCES nutrients(id) ON DELETE RESTRICT,
    target_value    DOUBLE PRECISION NOT NULL CHECK (target_value > 0),
    PRIMARY KEY (goal_profile_id, nutrient_id)
);
CREATE INDEX IF NOT EXISTS nutrient_goals_nutrient_idx
    ON nutrient_goals(nutrient_id);

CREATE TABLE IF NOT EXISTS nutrient_source_keys (
    source       TEXT NOT NULL CHECK (source IN ('fineli', 'open_food_facts')),
    source_key   TEXT NOT NULL,
    source_name  TEXT NOT NULL DEFAULT '',
    nutrient_id  UUID NOT NULL REFERENCES nutrients(id) ON DELETE RESTRICT,
    source_unit  TEXT NOT NULL,
    PRIMARY KEY (source, source_key)
);
ALTER TABLE nutrient_source_keys
    ADD COLUMN IF NOT EXISTS source_name TEXT NOT NULL DEFAULT '';
CREATE INDEX IF NOT EXISTS nutrient_source_keys_nutrient_idx
    ON nutrient_source_keys(nutrient_id);

CREATE TABLE IF NOT EXISTS foods (
    id             UUID PRIMARY KEY DEFAULT uuidv7(),
    owner_user_id  UUID REFERENCES users(id) ON DELETE CASCADE,
    source         TEXT NOT NULL CHECK (source IN ('fineli', 'open_food_facts', 'custom')),
    source_id      TEXT,
    display_name   TEXT NOT NULL CHECK (display_name <> ''),
    brand          TEXT,
    basis_unit     TEXT NOT NULL CHECK (basis_unit IN ('g', 'ml', 'count')),
    source_data    JSONB NOT NULL DEFAULT '{}'::jsonb,
    search_vector     TSVECTOR NOT NULL DEFAULT ''::tsvector,
    search_fi_vector  TSVECTOR NOT NULL DEFAULT ''::tsvector,
    search_sv_vector  TSVECTOR NOT NULL DEFAULT ''::tsvector,
    search_en_vector  TSVECTOR NOT NULL DEFAULT ''::tsvector,
    is_archived       BOOLEAN NOT NULL DEFAULT false,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (
        (source = 'custom' AND owner_user_id IS NOT NULL AND source_id IS NULL)
        OR
        (source IN ('fineli', 'open_food_facts') AND owner_user_id IS NULL AND source_id IS NOT NULL)
    )
);
CREATE UNIQUE INDEX IF NOT EXISTS foods_source_id_idx
    ON foods(source, source_id)
    WHERE source_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS foods_owner_idx ON foods(owner_user_id)
    WHERE owner_user_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS foods_display_name_idx ON foods(lower(display_name));
CREATE INDEX IF NOT EXISTS foods_search_vector_idx ON foods USING GIN(search_vector);
CREATE INDEX IF NOT EXISTS foods_search_fi_vector_idx ON foods USING GIN(search_fi_vector);
CREATE INDEX IF NOT EXISTS foods_search_sv_vector_idx ON foods USING GIN(search_sv_vector);
CREATE INDEX IF NOT EXISTS foods_search_en_vector_idx ON foods USING GIN(search_en_vector);

CREATE TABLE IF NOT EXISTS food_aliases (
    id         UUID PRIMARY KEY DEFAULT uuidv7(),
    food_id    UUID NOT NULL REFERENCES foods(id) ON DELETE CASCADE,
    name       TEXT NOT NULL CHECK (name <> ''),
    locale     TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX IF NOT EXISTS food_aliases_unique_idx
    ON food_aliases(food_id, lower(name), COALESCE(locale, ''));
CREATE INDEX IF NOT EXISTS food_aliases_name_idx ON food_aliases(lower(name));

CREATE OR REPLACE FUNCTION refresh_food_search_vector(target_food_id UUID)
RETURNS VOID
LANGUAGE SQL
AS $$
    UPDATE foods
    SET search_vector =
            setweight(to_tsvector('simple', replace(display_name, '/', ' ')), 'A')
            || setweight(to_tsvector('simple', COALESCE(brand, '')), 'B')
            || setweight(to_tsvector('simple', COALESCE((
                SELECT string_agg(replace(name, '/', ' '), ' ')
                FROM food_aliases WHERE food_id = target_food_id
            ), '')), 'C'),
        search_fi_vector = to_tsvector('finnish', COALESCE((
            SELECT string_agg(replace(name, '/', ' '), ' ') FROM food_aliases
            WHERE food_id = target_food_id AND locale = 'fi'
        ), '')),
        search_sv_vector = to_tsvector('swedish', COALESCE((
            SELECT string_agg(replace(name, '/', ' '), ' ') FROM food_aliases
            WHERE food_id = target_food_id AND locale = 'sv'
        ), '')),
        search_en_vector = to_tsvector('english', COALESCE((
            SELECT string_agg(replace(name, '/', ' '), ' ') FROM food_aliases
            WHERE food_id = target_food_id AND locale = 'en'
        ), ''))
    WHERE id = target_food_id;
$$;

CREATE OR REPLACE FUNCTION refresh_food_search_from_food()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM refresh_food_search_vector(NEW.id);
    RETURN NEW;
END;
$$;

CREATE OR REPLACE FUNCTION refresh_food_search_from_alias()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        PERFORM refresh_food_search_vector(OLD.food_id);
        RETURN OLD;
    END IF;
    IF TG_OP = 'UPDATE' AND OLD.food_id <> NEW.food_id THEN
        PERFORM refresh_food_search_vector(OLD.food_id);
    END IF;
    PERFORM refresh_food_search_vector(NEW.food_id);
    RETURN NEW;
END;
$$;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgname = 'foods_search_vector_trigger') THEN
        CREATE TRIGGER foods_search_vector_trigger
        AFTER INSERT OR UPDATE OF display_name, brand ON foods
        FOR EACH ROW EXECUTE FUNCTION refresh_food_search_from_food();
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgname = 'food_aliases_search_vector_trigger') THEN
        CREATE TRIGGER food_aliases_search_vector_trigger
        AFTER INSERT OR UPDATE OR DELETE ON food_aliases
        FOR EACH ROW EXECUTE FUNCTION refresh_food_search_from_alias();
    END IF;
END;
$$;

SELECT refresh_food_search_vector(id)
FROM foods
WHERE search_vector = ''::tsvector;

CREATE TABLE IF NOT EXISTS food_nutrients (
    food_id      UUID NOT NULL REFERENCES foods(id) ON DELETE CASCADE,
    nutrient_id  UUID NOT NULL REFERENCES nutrients(id) ON DELETE RESTRICT,
    value        DOUBLE PRECISION NOT NULL
                 CHECK (value >= 0 AND value < 'Infinity'::double precision),
    PRIMARY KEY (food_id, nutrient_id)
);
CREATE INDEX IF NOT EXISTS food_nutrients_nutrient_idx
    ON food_nutrients(nutrient_id);

CREATE TABLE IF NOT EXISTS food_shortcuts (
    id         UUID PRIMARY KEY DEFAULT uuidv7(),
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    food_id    UUID NOT NULL REFERENCES foods(id) ON DELETE CASCADE,
    name       TEXT NOT NULL CHECK (name <> ''),
    amount     DOUBLE PRECISION NOT NULL
               CHECK (amount > 0 AND amount < 'Infinity'::double precision),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX IF NOT EXISTS food_shortcuts_name_idx
    ON food_shortcuts(user_id, food_id, lower(name));

CREATE TABLE IF NOT EXISTS meals (
    id         UUID PRIMARY KEY DEFAULT uuidv7(),
    user_id    UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name       TEXT CHECK (name IS NULL OR btrim(name) <> ''),
    started_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS meals_user_started_idx
    ON meals(user_id, started_at DESC);

CREATE TABLE IF NOT EXISTS food_entries (
    id                    UUID PRIMARY KEY DEFAULT uuidv7(),
    user_id               UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    meal_id               UUID REFERENCES meals(id) ON DELETE SET NULL,
    food_id               UUID REFERENCES foods(id) ON DELETE SET NULL,
    amount                DOUBLE PRECISION NOT NULL
                          CHECK (amount > 0 AND amount < 'Infinity'::double precision),
    unit                  TEXT NOT NULL CHECK (unit IN ('g', 'ml', 'count')),
    eaten_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    food_name             TEXT NOT NULL,
    food_brand            TEXT,
    food_source           TEXT NOT NULL CHECK (food_source IN ('fineli', 'open_food_facts', 'custom')),
    food_source_id        TEXT,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now()
);
ALTER TABLE food_entries
    ADD COLUMN IF NOT EXISTS meal_id UUID REFERENCES meals(id) ON DELETE SET NULL;
ALTER TABLE food_entries DROP COLUMN IF EXISTS food_basis_unit;
CREATE INDEX IF NOT EXISTS food_entries_user_eaten_idx
    ON food_entries(user_id, eaten_at DESC);
CREATE INDEX IF NOT EXISTS food_entries_food_idx ON food_entries(food_id);
CREATE INDEX IF NOT EXISTS food_entries_meal_idx ON food_entries(meal_id);

CREATE TABLE IF NOT EXISTS food_entry_nutrients (
    food_entry_id  UUID NOT NULL REFERENCES food_entries(id) ON DELETE CASCADE,
    nutrient_id    UUID NOT NULL REFERENCES nutrients(id) ON DELETE RESTRICT,
    basis_value    DOUBLE PRECISION NOT NULL
                   CHECK (basis_value >= 0 AND basis_value < 'Infinity'::double precision),
    consumed_value DOUBLE PRECISION NOT NULL
                   CHECK (consumed_value >= 0 AND consumed_value < 'Infinity'::double precision),
    PRIMARY KEY (food_entry_id, nutrient_id)
);
CREATE INDEX IF NOT EXISTS food_entry_nutrients_nutrient_idx
    ON food_entry_nutrients(nutrient_id);
