-- Add up migration script here
CREATE TABLE IF NOT EXISTS 
    idps (
        id BINARY(16) PRIMARY KEY,
        idp_name VARCHAR(255) NOT NULL,
        idp_display_name VARCHAR(255) NOT NULL,
        idp_logo_svg TEXT,
        oidc_config_url TEXT NOT NULL,
        client_id TEXT NOT NULL,
        client_secret TEXT NOT NULL,
        enabled BOOLEAN NOT NULL
    );

CREATE TABLE IF NOT EXISTS 
    users (
        id BINARY(16) PRIMARY KEY,
        user_name VARCHAR(255) NOT NULL,
        enabled BOOLEAN NOT NULL DEFAULT TRUE,
        created_at DATETIME(3) NOT NULL,
        updated_at DATETIME(3) NOT NULL
    );

CREATE TABLE IF NOT EXISTS 
    user_idp_bindings (
        id BINARY(16) PRIMARY KEY,
        user_id BINARY(16) NOT NULL,
        idp_id BINARY(16) NOT NULL,
        idp_sub VARCHAR(255) NOT NULL,
        created_at DATETIME(3) NOT NULL,
        updated_at DATETIME(3) NOT NULL,
        FOREIGN KEY (user_id) REFERENCES users (id),
        FOREIGN KEY (idp_id) REFERENCES idps (id),
        UNIQUE (user_id, idp_id),
        INDEX (idp_id),
        INDEX (user_id)
    );

CREATE TABLE IF NOT EXISTS
    user_authed_tokens (
        id BINARY(16) PRIMARY KEY,
        user_id BINARY(16) NOT NULL,
        authed_token_id BINARY(16) NOT NULL,
        created_at DATETIME(3) NOT NULL,
        updated_at DATETIME(3) NOT NULL,
        FOREIGN KEY (user_id) REFERENCES users (id),
        FOREIGN KEY (authed_token_id) REFERENCES authed_tokens (id),
        UNIQUE (user_id, authed_token_id),
        INDEX (authed_token_id),
        INDEX (user_id)
    );

-- Use generated columns with non-nullable columns if peformance problems arise?
ALTER TABLE authed_tokens ADD COLUMN require_user_registration
    BOOLEAN NOT NULL DEFAULT FALSE;

ALTER TABLE authed_tokens ADD COLUMN registered_user_id
    BINARY(16);

ALTER TABLE authed_tokens ADD CONSTRAINT `authed_tokens_registered_user_id_foreign` 
    FOREIGN KEY (registered_user_id)
    REFERENCES users (id);
