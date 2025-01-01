-- Add down migration script here
ALTER TABLE authed_tokens ADD COLUMN author_id_seed BINARY(64) NOT NULL;
UPDATE authed_tokens SET author_id_seed = sha2(reduced_origin_ip, 256);
