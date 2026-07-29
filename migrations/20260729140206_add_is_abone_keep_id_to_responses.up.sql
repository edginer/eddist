-- Add up migration script here
ALTER TABLE responses ADD COLUMN is_abone_keep_id BOOLEAN NOT NULL DEFAULT FALSE, ALGORITHM=INSTANT;
