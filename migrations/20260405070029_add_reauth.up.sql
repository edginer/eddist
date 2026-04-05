ALTER TABLE authed_tokens
    ADD COLUMN require_reauth BOOLEAN NOT NULL DEFAULT FALSE;

ALTER TABLE captcha_configs
    ADD COLUMN endpoint_usage VARCHAR(16) NOT NULL DEFAULT 'auth_code';
