-- PostgreSQL equivalent
-- BINARY(16) → UUID, DATETIME(3) → TIMESTAMPTZ
-- JSON → JSONB, BOOLEAN DEFAULT 1 → DEFAULT TRUE
-- Inline INDEX → separate CREATE INDEX

CREATE TABLE IF NOT EXISTS captcha_configs (
    id                      UUID PRIMARY KEY,
    name                    VARCHAR(100) NOT NULL,
    provider                VARCHAR(50) NOT NULL,
    site_key                TEXT NOT NULL,
    secret                  TEXT NOT NULL,
    base_url                VARCHAR(255),
    -- Widget config fields (nullable for first-class providers which use defaults)
    widget_form_field_name  VARCHAR(100),
    widget_script_url       TEXT,
    widget_html             TEXT,
    widget_script_handler   TEXT,
    -- Capture fields as JSON array
    capture_fields          JSONB,
    -- Verification config as JSON (nullable for first-class providers)
    verification            JSONB,
    -- Metadata
    is_active               BOOLEAN NOT NULL DEFAULT TRUE,
    display_order           INT NOT NULL DEFAULT 0,
    created_at              TIMESTAMPTZ NOT NULL,
    updated_at              TIMESTAMPTZ NOT NULL,
    updated_by              VARCHAR(255)
);
CREATE INDEX idx_captcha_configs_provider ON captcha_configs (provider);
CREATE INDEX idx_captcha_configs_is_active ON captcha_configs (is_active);
CREATE INDEX idx_captcha_configs_display_order ON captcha_configs (display_order);
