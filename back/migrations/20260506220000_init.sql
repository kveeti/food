CREATE TABLE settings (
    id                      INTEGER PRIMARY KEY CHECK (id = 1),
    meal_interval_minutes   INTEGER NOT NULL CHECK (meal_interval_minutes > 0),
    reminder_offset_minutes INTEGER NOT NULL CHECK (reminder_offset_minutes >= 0),
    meals                   TEXT NOT NULL,
    reminders_paused        INTEGER NOT NULL DEFAULT 0 CHECK (reminders_paused IN (0, 1)),
    timezone                TEXT NOT NULL,
    updated_at              TEXT NOT NULL
);

CREATE TABLE meals (
    id         INTEGER PRIMARY KEY,
    meal_type  TEXT NOT NULL,
    started_at TEXT NOT NULL,
    ended_at   TEXT,
    note       TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (ended_at IS NULL OR ended_at >= started_at)
);

CREATE INDEX idx_meals_started_at ON meals(started_at);
CREATE INDEX idx_meals_open ON meals(ended_at) WHERE ended_at IS NULL;

CREATE TABLE push_subscriptions (
    id         INTEGER PRIMARY KEY,
    endpoint   TEXT NOT NULL UNIQUE,
    p256dh     TEXT NOT NULL,
    auth       TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE notification_log (
    id                     INTEGER PRIMARY KEY,
    kind                   TEXT NOT NULL,
    target_at              TEXT NOT NULL,
    sent_at                TEXT NOT NULL,
    meals_started_for_day  INTEGER NOT NULL,
    UNIQUE(kind, target_at)
);
