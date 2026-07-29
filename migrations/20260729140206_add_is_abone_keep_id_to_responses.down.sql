-- Add down migration script here
ALTER TABLE responses DROP COLUMN is_abone_keep_id, ALGORITHM=INSTANT;
