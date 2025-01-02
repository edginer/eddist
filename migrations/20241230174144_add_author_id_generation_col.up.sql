-- Add up migration script here
ALTER TABLE authed_tokens ADD COLUMN author_id_seed BINARY(64) NOT NULL DEFAULT 0x0;
UPDATE authed_tokens SET author_id_seed = unhex(sha2(reduced_origin_ip, 512));