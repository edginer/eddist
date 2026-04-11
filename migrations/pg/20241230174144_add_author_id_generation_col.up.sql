-- PostgreSQL equivalent
-- BINARY(64) → BYTEA
-- unhex(sha2(..., 512)) → digest(..., 'sha512') via pgcrypto
CREATE EXTENSION IF NOT EXISTS pgcrypto;

ALTER TABLE authed_tokens ADD COLUMN author_id_seed BYTEA NOT NULL DEFAULT '\x00'::bytea;
UPDATE authed_tokens SET author_id_seed = digest(reduced_origin_ip, 'sha512');
