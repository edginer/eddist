-- Identical to MySQL (standard SQL; JSON → JSONB for better indexing in PostgreSQL)
ALTER TABLE authed_tokens
    ADD COLUMN asn_num INT NOT NULL DEFAULT 0,
    ADD COLUMN additional_info JSONB;
