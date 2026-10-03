-- Drop all tables and types in reverse dependency order.

-- Tables with no dependents first
DROP TABLE IF EXISTS server_settings;
DROP TABLE IF EXISTS captcha_configs;
DROP TABLE IF EXISTS notices;
DROP TABLE IF EXISTS terms;

-- user_restriction_rules + its enum type
DROP TABLE IF EXISTS user_restriction_rules;
DROP TYPE IF EXISTS restriction_rule_type;

-- user linking tables
DROP TABLE IF EXISTS user_authed_tokens;
DROP TABLE IF EXISTS user_idp_bindings;

-- Remove FK from authed_tokens → users before dropping users
ALTER TABLE IF EXISTS authed_tokens DROP CONSTRAINT IF EXISTS authed_tokens_registered_user_id_foreign;
DROP TABLE IF EXISTS users;
DROP TABLE IF EXISTS idps;

-- Core BBS tables (responses before threads, threads before boards/authed_tokens)
DROP TABLE IF EXISTS responses;
DROP TABLE IF EXISTS threads;

-- Association tables
DROP TABLE IF EXISTS boards_caps;
DROP TABLE IF EXISTS boards_ng_words;
DROP TABLE IF EXISTS caps;
DROP TABLE IF EXISTS ng_words;

-- Admin tables
DROP TABLE IF EXISTS admin_role_scopes;
DROP TABLE IF EXISTS admin_users;
DROP TABLE IF EXISTS admin_roles;

DROP TABLE IF EXISTS boards_info;
DROP TABLE IF EXISTS authed_tokens;
DROP TABLE IF EXISTS boards;

-- Partitioned tables: dropping the parent also drops all partition children in PostgreSQL
DROP TABLE IF EXISTS archived_responses;
DROP TABLE IF EXISTS archived_threads;

DROP EXTENSION IF EXISTS pgcrypto;
