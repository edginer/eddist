-- PostgreSQL equivalent
-- BINARY(16) → UUID
-- DATETIME(3) → TIMESTAMPTZ
-- Backtick identifiers → double-quote identifiers
-- Inline INDEX → separate CREATE INDEX

CREATE TABLE IF NOT EXISTS idps (
    id                  UUID PRIMARY KEY,
    idp_name            VARCHAR(255) NOT NULL,
    idp_display_name    VARCHAR(255) NOT NULL,
    idp_logo_svg        TEXT,
    oidc_config_url     TEXT NOT NULL,
    client_id           TEXT NOT NULL,
    client_secret       TEXT NOT NULL,
    enabled             BOOLEAN NOT NULL
);

CREATE TABLE IF NOT EXISTS users (
    id          UUID PRIMARY KEY,
    user_name   VARCHAR(255) NOT NULL,
    enabled     BOOLEAN NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ NOT NULL,
    updated_at  TIMESTAMPTZ NOT NULL
);

CREATE TABLE IF NOT EXISTS user_idp_bindings (
    id          UUID PRIMARY KEY,
    user_id     UUID NOT NULL,
    idp_id      UUID NOT NULL,
    idp_sub     VARCHAR(255) NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL,
    updated_at  TIMESTAMPTZ NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users (id),
    FOREIGN KEY (idp_id) REFERENCES idps (id),
    UNIQUE (user_id, idp_id)
);
CREATE INDEX ON user_idp_bindings (idp_id);
CREATE INDEX ON user_idp_bindings (user_id);

CREATE TABLE IF NOT EXISTS user_authed_tokens (
    id              UUID PRIMARY KEY,
    user_id         UUID NOT NULL,
    authed_token_id UUID NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL,
    updated_at      TIMESTAMPTZ NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users (id),
    FOREIGN KEY (authed_token_id) REFERENCES authed_tokens (id),
    UNIQUE (user_id, authed_token_id)
);
CREATE INDEX ON user_authed_tokens (authed_token_id);
CREATE INDEX ON user_authed_tokens (user_id);

ALTER TABLE authed_tokens ADD COLUMN require_user_registration
    BOOLEAN NOT NULL DEFAULT FALSE;

ALTER TABLE authed_tokens ADD COLUMN registered_user_id UUID;

ALTER TABLE authed_tokens ADD CONSTRAINT "authed_tokens_registered_user_id_foreign"
    FOREIGN KEY (registered_user_id) REFERENCES users (id);
