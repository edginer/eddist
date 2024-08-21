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
    name VARCHAR(255) NOT NULL,
    board_key VARCHAR(255) UNIQUE NOT NULL,
    default_name TEXT NOT NULL
);

CREATE TABLE boards_info (
    -- id is same as board_id
    id BINARY(16) PRIMARY KEY,
    local_rules TEXT NOT NULL,
    base_thread_creation_span_sec INT NOT NULL DEFAULT 120,
    base_response_creation_span_sec INT NOT NULL DEFAULT 5,
    max_thread_name_byte_length INT NOT NULL DEFAULT 256,
    max_author_name_byte_length INT NOT NULL DEFAULT 128,
    max_email_byte_length INT NOT NULL DEFAULT 128,
    max_response_body_byte_length INT NOT NULL DEFAULT 9192,
    max_response_body_lines INT NOT NULL DEFAULT 32,
    threads_archive_cron VARCHAR(255),
    threads_archive_trigger_thread_count INT,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    FOREIGN KEY (id) REFERENCES boards(id)
);

CREATE TABLE threads (
    id BINARY(16) PRIMARY KEY,
    board_id BINARY(16) NOT NULL,
    thread_number BIGINT NOT NULL,
    last_modified_at DATETIME(3) NOT NULL,
    sage_last_modified_at DATETIME(3) NOT NULL,
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
    created_at DATETIME(3) NOT NULL,
    author_id TEXT NOT NULL,
    ip_addr TEXT NOT NULL,
    authed_token_id BINARY(16) NOT NULL,
    board_id BINARY(16) NOT NULL,
    thread_id BINARY(16) NOT NULL,
    is_abone BOOLEAN NOT NULL DEFAULT FALSE,
    res_order INTEGER NOT NULL,
    client_info JSON NOT NULL,
    FOREIGN KEY (board_id) REFERENCES boards(id),
    FOREIGN KEY (thread_id) REFERENCES threads(id) ON DELETE CASCADE,
    INDEX (thread_id)
);

CREATE TABLE caps (
    id BINARY(16) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL
);

CREATE TABLE boards_caps (
    id BINARY(16) PRIMARY KEY,
    board_id BINARY(16) NOT NULL,
    cap_id BINARY(16) NOT NULL,
    FOREIGN KEY (board_id) REFERENCES boards(id),
    FOREIGN KEY (cap_id) REFERENCES caps(id)
);

CREATE TABLE ng_words (
    id BINARY(16) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    word VARCHAR(255) NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL
);

CREATE TABLE boards_ng_words (
    id BINARY(16) PRIMARY KEY,
    board_id BINARY(16) NOT NULL,
    ng_word_id BINARY(16) NOT NULL,
    FOREIGN KEY (board_id) REFERENCES boards(id),
    FOREIGN KEY (ng_word_id) REFERENCES ng_words(id)
);

CREATE TABLE admin_roles (
    id BINARY(16) PRIMARY KEY,
    role_name VARCHAR(255) NOT NULL,
    role_description TEXT NOT NULL
);

CREATE TABLE admin_role_scopes (
    id BINARY(16) PRIMARY KEY,
    role_id BINARY(16) NOT NULL,
    scope_key VARCHAR(255) NOT NULL,
    FOREIGN KEY (role_id) REFERENCES admin_roles(id),
    INDEX (role_id)
);

CREATE TABLE admin_users (
    id BINARY(16) PRIMARY KEY,
    user_role_id BINARY(16) NOT NULL,
    FOREIGN KEY (user_role_id) REFERENCES admin_roles(id)
);

INSERT INTO
    boards (
        id,
        name,
        board_key,
        default_name
    )
VALUES
    (
        UUID_TO_BIN('01815522-2d2b-728f-af94-a234aabb6b20'),
        'vエッヂk',
        'livedgek',
        'ポッドの名無し'
    );

INSERT INTO
    boards_info (
        id,
        local_rules,
        created_at,
        updated_at
    )
VALUES
    (
        UUID_TO_BIN('01815522-2d2b-728f-af94-a234aabb6b20'),
        'ローカルルール',
        NOW(),
        NOW()
    );