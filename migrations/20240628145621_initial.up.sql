-- Add up migration script here
CREATE TABLE authed_tokens (
    id BINARY(16) PRIMARY KEY,
    token VARCHAR(255) UNIQUE NOT NULL,
    origin_ip VARCHAR(255) NOT NULL,
    reduced_origin_ip VARCHAR(255) NOT NULL,
    writing_ua TEXT NOT NULL,
    authed_ua TEXT,
    auth_code VARCHAR(12) NOT NULL,
    created_at TIMESTAMP NOT NULL,
    authed_at TIMESTAMP NULL,
    validity BOOLEAN NOT NULL,
    INDEX (origin_ip),
    INDEX (auth_code)
);

CREATE TABLE boards (
    id BINARY(16) PRIMARY KEY,
    name TEXT NOT NULL,
    board_key VARCHAR(255) UNIQUE NOT NULL,
    local_rule TEXT NOT NULL,
    default_name TEXT NOT NULL
);

CREATE TABLE threads (
    id BINARY(16) PRIMARY KEY,
    board_id BINARY(16) NOT NULL,
    thread_number BIGINT NOT NULL,
    last_modified_at TIMESTAMP NOT NULL,
    sage_last_modified_at TIMESTAMP NOT NULL,
    title TEXT NOT NULL,
    authed_token_id BINARY(16) NOT NULL,
    metadent TEXT NOT NULL,
    response_count INT NOT NULL,
    no_pool BOOLEAN NOT NULL DEFAULT FALSE,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    archived BOOLEAN NOT NULL DEFAULT FALSE,
    FOREIGN KEY (board_id) REFERENCES boards(id),
    FOREIGN KEY (authed_token_id) REFERENCES authed_tokens(id),
    INDEX (thread_number),
    UNIQUE (board_id, thread_number)
);

CREATE TABLE responses (
    id BINARY(16) PRIMARY KEY,
    author_name TEXT NOT NULL,
    mail TEXT NOT NULL,
    body TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    author_id TEXT NOT NULL,
    ip_addr TEXT NOT NULL,
    authed_token_id BINARY(16) NOT NULL,
    board_id BINARY(16) NOT NULL,
    thread_id BINARY(16) NOT NULL,
    is_abone BOOLEAN NOT NULL DEFAULT FALSE,
    res_order INTEGER NOT NULL,
    FOREIGN KEY (board_id) REFERENCES boards(id),
    FOREIGN KEY (thread_id) REFERENCES threads(id) ON DELETE CASCADE,
    INDEX (thread_id)
);

CREATE TABLE caps (
    id BINARY(16) PRIMARY KEY,
    cap_name TEXT NOT NULL,
    cap_password_hash TEXT NOT NULL
);