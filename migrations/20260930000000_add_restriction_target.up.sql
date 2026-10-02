ALTER TABLE user_restriction_rules
    ADD COLUMN target VARCHAR(32) NOT NULL DEFAULT 'BOTH';
