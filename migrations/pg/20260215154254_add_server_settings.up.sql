-- PostgreSQL equivalent
-- BINARY(16) → UUID, DATETIME(3) → TIMESTAMPTZ
-- Inline INDEX → separate CREATE INDEX

CREATE TABLE IF NOT EXISTS server_settings (
    id          UUID PRIMARY KEY,
    setting_key VARCHAR(255) NOT NULL UNIQUE,
    value       TEXT NOT NULL,
    description TEXT,
    created_at  TIMESTAMPTZ NOT NULL,
    updated_at  TIMESTAMPTZ NOT NULL
);
CREATE INDEX idx_server_settings_key ON server_settings (setting_key);
