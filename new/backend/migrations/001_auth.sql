CREATE TABLE users (
    id uuid PRIMARY KEY,
    issuer text NOT NULL,
    subject text NOT NULL,
    email text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (issuer, subject)
);

CREATE TABLE sessions (
    id uuid PRIMARY KEY,
    token_hash bytea NOT NULL UNIQUE,
    user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    access_token bytea NOT NULL,
    refresh_token bytea NOT NULL,
    oidc_session_id text NOT NULL,
    refresh_retry_after timestamptz,
    refresh_expires_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX sessions_user_idx ON sessions (user_id);
CREATE INDEX sessions_oidc_session_idx ON sessions (oidc_session_id);
CREATE INDEX sessions_refresh_expires_idx ON sessions (refresh_expires_at);
