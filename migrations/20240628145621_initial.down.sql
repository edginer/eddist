-- Add down migration script here
DROP TABLE IF EXISTS responses;

DROP TABLE IF EXISTS threads;

DROP TABLE IF EXISTS boards_ng_words;

DROP TABLE IF EXISTS boards_caps;

DROP TABLE IF EXISTS caps;

DROP TABLE IF EXISTS boards_info;

DROP TABLE IF EXISTS boards;

DROP TABLE IF EXISTS authed_tokens;

DROP TABLE IF EXISTS ng_words;

DROP TABLE IF EXISTS admin_users;

DROP TABLE IF EXISTS admin_role_scopes;

DROP TABLE IF EXISTS admin_roles;

DROP TABLE IF EXISTS archived_threads;

DROP TABLE IF EXISTS archived_responses;
