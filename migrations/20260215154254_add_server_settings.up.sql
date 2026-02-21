CREATE TABLE IF NOT EXISTS server_settings (
    id BINARY(16) PRIMARY KEY,
    setting_key VARCHAR(255) NOT NULL UNIQUE,
    value TEXT NOT NULL,
    description TEXT,
    created_at DATETIME(3) NOT NULL,
    updated_at DATETIME(3) NOT NULL,
    INDEX idx_server_settings_key (setting_key)
);
