-- Add up migration script here
CREATE TABLE
    authed_tokens (
        id BINARY(16) PRIMARY KEY,
        token VARCHAR(255) UNIQUE NOT NULL,
        origin_ip VARCHAR(255) NOT NULL,
        reduced_origin_ip VARCHAR(255) NOT NULL,
        writing_ua TEXT NOT NULL,
        authed_ua TEXT,
        auth_code VARCHAR(12) NOT NULL,
        created_at DATETIME(3) NOT NULL,
        authed_at DATETIME(3) NULL,
        validity BOOLEAN NOT NULL,
        INDEX (origin_ip),
        INDEX (auth_code)
    );

CREATE TABLE
    boards (
        id BINARY(16) PRIMARY KEY,
        name VARCHAR(255) NOT NULL,
        board_key VARCHAR(255) UNIQUE NOT NULL,
        default_name TEXT NOT NULL
    );

CREATE TABLE
    boards_info (
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
        created_at DATETIME(3) NOT NULL,
        updated_at DATETIME(3) NOT NULL,
        FOREIGN KEY (id) REFERENCES boards (id)
    );

CREATE TABLE
    threads (
        id BINARY(16) PRIMARY KEY,
        board_id BINARY(16) NOT NULL,
        thread_number BIGINT NOT NULL,
        last_modified_at DATETIME (3) NOT NULL,
        sage_last_modified_at DATETIME (3) NOT NULL,
        title TEXT NOT NULL,
        authed_token_id BINARY(16) NOT NULL,
        metadent TEXT NOT NULL,
        response_count INT NOT NULL,
        no_pool BOOLEAN NOT NULL DEFAULT FALSE,
        active BOOLEAN NOT NULL DEFAULT TRUE,
        archived BOOLEAN NOT NULL DEFAULT FALSE,
        archive_converted BOOLEAN NOT NULL DEFAULT FALSE,
        FOREIGN KEY (board_id) REFERENCES boards (id),
        FOREIGN KEY (authed_token_id) REFERENCES authed_tokens (id),
        INDEX (thread_number),
        UNIQUE (board_id, thread_number)
    );

CREATE TABLE
    responses (
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
        FOREIGN KEY (board_id) REFERENCES boards (id),
        FOREIGN KEY (thread_id) REFERENCES threads (id) ON DELETE CASCADE,
        INDEX (thread_id)
    );

CREATE TABLE
    caps (
        id BINARY(16) PRIMARY KEY,
        name VARCHAR(255) NOT NULL,
        description TEXT NOT NULL,
        password_hash VARCHAR(255) NOT NULL,
        created_at DATETIME(3) NOT NULL,
        updated_at DATETIME(3) NOT NULL
    );

CREATE TABLE
    boards_caps (
        id BINARY(16) PRIMARY KEY,
        board_id BINARY(16) NOT NULL,
        cap_id BINARY(16) NOT NULL,
        FOREIGN KEY (board_id) REFERENCES boards (id),
        FOREIGN KEY (cap_id) REFERENCES caps (id)
    );

CREATE TABLE
    ng_words (
        id BINARY(16) PRIMARY KEY,
        name VARCHAR(255) NOT NULL,
        word VARCHAR(255) NOT NULL,
        created_at DATETIME(3) NOT NULL,
        updated_at DATETIME(3) NOT NULL
    );

CREATE TABLE
    boards_ng_words (
        id BINARY(16) PRIMARY KEY,
        board_id BINARY(16) NOT NULL,
        ng_word_id BINARY(16) NOT NULL,
        FOREIGN KEY (board_id) REFERENCES boards (id),
        FOREIGN KEY (ng_word_id) REFERENCES ng_words (id)
    );

CREATE TABLE
    admin_roles (id BINARY(16) PRIMARY KEY, role_name VARCHAR(255) NOT NULL, role_description TEXT NOT NULL);

CREATE TABLE
    admin_role_scopes (
        id BINARY(16) PRIMARY KEY,
        role_id BINARY(16) NOT NULL,
        scope_key VARCHAR(255) NOT NULL,
        FOREIGN KEY (role_id) REFERENCES admin_roles (id),
        INDEX (role_id)
    );

CREATE TABLE
    admin_users (id BINARY(16) PRIMARY KEY, user_role_id BINARY(16) NOT NULL, FOREIGN KEY (user_role_id) REFERENCES admin_roles (id));

CREATE TABLE
    archived_threads (
        id BINARY(16),
        board_id BINARY(16) NOT NULL,
        thread_number BIGINT NOT NULL,
        last_modified_at DATETIME (3) NOT NULL,
        sage_last_modified_at DATETIME (3) NOT NULL,
        title TEXT NOT NULL,
        authed_token_id BINARY(16) NOT NULL,
        metadent TEXT NOT NULL,
        response_count INT NOT NULL,
        no_pool BOOLEAN NOT NULL DEFAULT FALSE,
        active BOOLEAN NOT NULL DEFAULT TRUE,
        archived BOOLEAN NOT NULL DEFAULT FALSE,
        INDEX (thread_number),
        PRIMARY KEY (id, last_modified_at)
    )
PARTITION BY 
    RANGE (YEAR (last_modified_at)) (
        PARTITION p2024 VALUES LESS THAN (2025),
        PARTITION p2025 VALUES LESS THAN (2026),
        PARTITION p2026 VALUES LESS THAN (2027),
        PARTITION p2027 VALUES LESS THAN (2028),
        PARTITION max_value VALUES LESS THAN MAXVALUE
    );

CREATE TABLE
    archived_responses (
        id BINARY(16),
        author_name TEXT NOT NULL,
        mail TEXT NOT NULL,
        body TEXT NOT NULL,
        created_at DATETIME (3) NOT NULL,
        author_id TEXT NOT NULL,
        ip_addr TEXT NOT NULL,
        authed_token_id BINARY(16) NOT NULL,
        board_id BINARY(16) NOT NULL,
        thread_id BINARY(16) NOT NULL,
        is_abone BOOLEAN NOT NULL DEFAULT FALSE,
        res_order INTEGER NOT NULL,
        client_info JSON NOT NULL,
        INDEX (thread_id),
        PRIMARY KEY (id, created_at)
    )
PARTITION BY
    RANGE (YEAR (created_at) * 100 + MONTH (created_at)) (
        PARTITION p2024_9 VALUES LESS THAN (202410),
        PARTITION p2024_10 VALUES LESS THAN (202411),
        PARTITION p2024_11 VALUES LESS THAN (202412),
        PARTITION p2024_12 VALUES LESS THAN (202501),
        PARTITION p2025_1 VALUES LESS THAN (202502),
        PARTITION p2025_2 VALUES LESS THAN (202503),
        PARTITION p2025_3 VALUES LESS THAN (202504),
        PARTITION p2025_4 VALUES LESS THAN (202505),
        PARTITION p2025_5 VALUES LESS THAN (202506),
        PARTITION p2025_6 VALUES LESS THAN (202507),
        PARTITION p2025_7 VALUES LESS THAN (202508),
        PARTITION p2025_8 VALUES LESS THAN (202509),
        PARTITION p2025_9 VALUES LESS THAN (202510),
        PARTITION p2025_10 VALUES LESS THAN (202511),
        PARTITION p2025_11 VALUES LESS THAN (202512),
        PARTITION p2025_12 VALUES LESS THAN (202601),
        PARTITION p2026_1 VALUES LESS THAN (202602),
        PARTITION p2026_2 VALUES LESS THAN (202603),
        PARTITION p2026_3 VALUES LESS THAN (202604),
        PARTITION p2026_4 VALUES LESS THAN (202605),
        PARTITION p2026_5 VALUES LESS THAN (202606),
        PARTITION p2026_6 VALUES LESS THAN (202607),
        PARTITION p2026_7 VALUES LESS THAN (202608),
        PARTITION p2026_8 VALUES LESS THAN (202609),
        PARTITION p2026_9 VALUES LESS THAN (202610),
        PARTITION p2026_10 VALUES LESS THAN (202611),
        PARTITION p2026_11 VALUES LESS THAN (202612),
        PARTITION p2026_12 VALUES LESS THAN (202701),
        PARTITION p2027_1 VALUES LESS THAN (202702),
        PARTITION p2027_2 VALUES LESS THAN (202703),
        PARTITION p2027_3 VALUES LESS THAN (202704),
        PARTITION p2027_4 VALUES LESS THAN (202705),
        PARTITION p2027_5 VALUES LESS THAN (202706),
        PARTITION p2027_6 VALUES LESS THAN (202707),
        PARTITION p2027_7 VALUES LESS THAN (202708),
        PARTITION p2027_8 VALUES LESS THAN (202709),
        PARTITION p2027_9 VALUES LESS THAN (202710),
        PARTITION p2027_10 VALUES LESS THAN (202711),
        PARTITION p2027_11 VALUES LESS THAN (202712),
        PARTITION p2027_12 VALUES LESS THAN (202801),
        PARTITION max_value VALUES LESS THAN MAXVALUE
    );

INSERT INTO
    boards (id, name, board_key, default_name)
VALUES
    (UUID_TO_BIN ('01815522-2d2b-728f-af94-a234aabb6b20'), 'vエッヂk', 'livedgek', 'ポッドの名無し');

INSERT INTO
    boards_info (id, local_rules, created_at, updated_at)
VALUES
    (UUID_TO_BIN ('01815522-2d2b-728f-af94-a234aabb6b20'), 'ローカルルール', NOW (), NOW ());