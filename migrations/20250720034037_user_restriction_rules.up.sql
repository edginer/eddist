-- Create user_restriction_rules table
CREATE TABLE user_restriction_rules (
    id BINARY(16) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    rule_type ENUM('ASN', 'IP', 'IP_CIDR', 'USER_AGENT') NOT NULL,
    rule_value TEXT NOT NULL,
    expires_at DATETIME(3),
    created_at DATETIME(3) NOT NULL,
    updated_at DATETIME(3) NOT NULL,
    created_by_email VARCHAR(255) NOT NULL,
    INDEX (rule_type),
    INDEX (expires_at),
    INDEX (created_at)
);