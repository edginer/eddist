CREATE TABLE IF NOT EXISTS captcha_configs (
    id BINARY(16) PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    provider VARCHAR(50) NOT NULL,
    site_key TEXT NOT NULL,
    secret TEXT NOT NULL,
    base_url VARCHAR(255),
    -- Widget config fields (nullable for first-class providers which use defaults)
    widget_form_field_name VARCHAR(100),
    widget_script_url TEXT,
    widget_html TEXT,
    widget_script_handler TEXT,
    -- Capture fields as JSON array
    capture_fields JSON,
    -- Verification config as JSON (nullable for first-class providers)
    verification JSON,
    -- Metadata
    is_active BOOLEAN NOT NULL DEFAULT 1,
    display_order INT NOT NULL DEFAULT 0,
    created_at DATETIME(3) NOT NULL,
    updated_at DATETIME(3) NOT NULL,
    updated_by VARCHAR(255),
    INDEX idx_captcha_configs_provider (provider),
    INDEX idx_captcha_configs_is_active (is_active),
    INDEX idx_captcha_configs_display_order (display_order)
);
