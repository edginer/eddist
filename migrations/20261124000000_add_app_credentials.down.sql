DROP TABLE app_attest_keys;
DROP TABLE app_credentials;
UPDATE authed_tokens SET validity = FALSE WHERE is_app = TRUE;
ALTER TABLE authed_tokens DROP COLUMN is_app;
