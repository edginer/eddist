-- Add down migration script here
ALTER TABLE authed_tokens DROP FOREIGN KEY authed_tokens_registered_user_id_foreign;
ALTER TABLE authed_tokens DROP COLUMN require_user_registration; 
ALTER TABLE authed_tokens DROP COLUMN registered_user_id;

DROP TABLE IF EXISTS user_authed_tokens;
DROP TABLE IF EXISTS user_idp_bindings;
DROP TABLE IF EXISTS users;
DROP TABLE IF EXISTS idps;
