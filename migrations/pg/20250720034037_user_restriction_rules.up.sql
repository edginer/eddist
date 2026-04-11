-- PostgreSQL equivalent
-- MySQL inline ENUM → PostgreSQL CREATE TYPE ... AS ENUM
-- BINARY(16) → UUID
-- DATETIME(3) → TIMESTAMPTZ
-- Inline INDEX → separate CREATE INDEX

CREATE TYPE restriction_rule_type AS ENUM ('ASN', 'IP', 'IP_CIDR', 'USER_AGENT');

CREATE TABLE user_restriction_rules (
    id              UUID PRIMARY KEY,
    name            VARCHAR(255) NOT NULL,
    rule_type       restriction_rule_type NOT NULL,
    rule_value      TEXT NOT NULL,
    expires_at      TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL,
    updated_at      TIMESTAMPTZ NOT NULL,
    created_by_email VARCHAR(255) NOT NULL
);
CREATE INDEX ON user_restriction_rules (rule_type);
CREATE INDEX ON user_restriction_rules (expires_at);
CREATE INDEX ON user_restriction_rules (created_at);
