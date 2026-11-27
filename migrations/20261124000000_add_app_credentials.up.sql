ALTER TABLE authed_tokens ADD COLUMN is_app BOOLEAN NOT NULL DEFAULT FALSE;

CREATE TABLE app_credentials (
    authed_token_id BINARY(16) PRIMARY KEY,
    platform VARCHAR(16) NOT NULL CHECK (platform IN ('ios', 'android')),
    scope_hash BINARY(32) NOT NULL,
    bearer_hash BINARY(32) NOT NULL UNIQUE,
    registration_key_hash BINARY(32) NOT NULL,
    registration_request_hash BINARY(32) NOT NULL,
    registered_at DATETIME(3) NOT NULL,
    verified_at DATETIME(3) NOT NULL,
    reattest_after DATETIME(3) NOT NULL,
    UNIQUE (scope_hash, registration_key_hash),
    FOREIGN KEY (authed_token_id) REFERENCES authed_tokens (id) ON DELETE CASCADE
);

CREATE TABLE app_attest_keys (
    authed_token_id BINARY(16) PRIMARY KEY,
    key_id BINARY(32) NOT NULL UNIQUE,
    public_key BINARY(65) NOT NULL,
    counter INT UNSIGNED NOT NULL,
    FOREIGN KEY (authed_token_id) REFERENCES app_credentials (authed_token_id) ON DELETE CASCADE
);
