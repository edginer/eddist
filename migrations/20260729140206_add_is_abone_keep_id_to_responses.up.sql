-- Add up migration script here
-- responses is a small, actively-churned table (only currently-active threads live here),
-- so this is safe as an instant, metadata-only DDL. archived_responses is intentionally NOT
-- touched: it is a huge table and the abone-keep-id feature doesn't need it there (the
-- S3-archived dat path is self-describing from the dat text itself).
ALTER TABLE responses ADD COLUMN is_abone_keep_id BOOLEAN NOT NULL DEFAULT FALSE, ALGORITHM=INSTANT;
