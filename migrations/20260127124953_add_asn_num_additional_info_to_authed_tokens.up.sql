-- Add asn_num and additional_info columns to authed_tokens table
ALTER TABLE authed_tokens
    ADD COLUMN asn_num INT NOT NULL DEFAULT 0,
    ADD COLUMN additional_info JSON NULL;
