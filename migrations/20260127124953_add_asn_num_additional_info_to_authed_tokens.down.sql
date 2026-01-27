-- Remove asn_num and additional_info columns from authed_tokens table
ALTER TABLE authed_tokens
    DROP COLUMN asn_num,
    DROP COLUMN additional_info;
