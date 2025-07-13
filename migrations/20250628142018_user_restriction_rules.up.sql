-- Create user_restriction_rules table
CREATE TABLE user_restriction_rules (
    id BINARY(16) NOT NULL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    filter_expression TEXT NOT NULL,
    restriction_type ENUM('creating_response', 'creating_thread', 'auth_code', 'all') NOT NULL,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    created_by VARCHAR(255),
    description TEXT,
    
    INDEX idx_user_restriction_rules_active (active),
    INDEX idx_user_restriction_rules_type (restriction_type),
    INDEX idx_user_restriction_rules_active_type (active, restriction_type),
    INDEX idx_user_restriction_rules_created_at (created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;