-- PostgreSQL equivalent of the initial migration
-- Key differences from MySQL:
--   BINARY(16) → UUID
--   UUID_TO_BIN() → cast literal '...'::uuid
--   DATETIME(3)  → TIMESTAMPTZ
--   Inline INDEX → separate CREATE INDEX statements
--   PARTITION BY RANGE (YEAR(...)) → PARTITION BY RANGE (EXTRACT(YEAR FROM ...))
--   archived_responses RANGE+HASH → RANGE by year (PostgreSQL has no subpartitions)

CREATE TABLE authed_tokens (
    id              UUID PRIMARY KEY,
    token           VARCHAR(255) UNIQUE NOT NULL,
    origin_ip       VARCHAR(255) NOT NULL,
    reduced_origin_ip VARCHAR(255) NOT NULL,
    writing_ua      TEXT NOT NULL,
    authed_ua       TEXT,
    auth_code       VARCHAR(12) NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL,
    authed_at       TIMESTAMPTZ,
    validity        BOOLEAN NOT NULL,
    last_wrote_at   TIMESTAMPTZ
);
CREATE INDEX ON authed_tokens (origin_ip);
CREATE INDEX ON authed_tokens (auth_code);

CREATE TABLE boards (
    id              UUID PRIMARY KEY,
    name            VARCHAR(255) NOT NULL,
    board_key       VARCHAR(255) UNIQUE NOT NULL,
    default_name    TEXT NOT NULL
);

CREATE TABLE boards_info (
    -- id is same as board_id
    id                                  UUID PRIMARY KEY,
    local_rules                         TEXT NOT NULL,
    base_thread_creation_span_sec       INT NOT NULL DEFAULT 120,
    base_response_creation_span_sec     INT NOT NULL DEFAULT 5,
    max_thread_name_byte_length         INT NOT NULL DEFAULT 256,
    max_author_name_byte_length         INT NOT NULL DEFAULT 128,
    max_email_byte_length               INT NOT NULL DEFAULT 128,
    max_response_body_byte_length       INT NOT NULL DEFAULT 9192,
    max_response_body_lines             INT NOT NULL DEFAULT 32,
    threads_archive_cron                VARCHAR(255),
    threads_archive_trigger_thread_count INT,
    read_only                           BOOLEAN NOT NULL DEFAULT FALSE,
    created_at                          TIMESTAMPTZ NOT NULL,
    updated_at                          TIMESTAMPTZ NOT NULL,
    FOREIGN KEY (id) REFERENCES boards (id)
);

CREATE TABLE threads (
    id                  UUID PRIMARY KEY,
    board_id            UUID NOT NULL,
    thread_number       BIGINT NOT NULL,
    last_modified_at    TIMESTAMPTZ NOT NULL,
    sage_last_modified_at TIMESTAMPTZ NOT NULL,
    title               TEXT NOT NULL,
    authed_token_id     UUID NOT NULL,
    metadent            TEXT NOT NULL,
    response_count      INT NOT NULL,
    no_pool             BOOLEAN NOT NULL DEFAULT FALSE,
    active              BOOLEAN NOT NULL DEFAULT TRUE,
    archived            BOOLEAN NOT NULL DEFAULT FALSE,
    archive_converted   BOOLEAN NOT NULL DEFAULT FALSE,
    FOREIGN KEY (board_id) REFERENCES boards (id),
    FOREIGN KEY (authed_token_id) REFERENCES authed_tokens (id),
    UNIQUE (board_id, thread_number)
);
CREATE INDEX ON threads (thread_number);

CREATE TABLE responses (
    id              UUID PRIMARY KEY,
    author_name     TEXT NOT NULL,
    mail            TEXT NOT NULL,
    body            TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL,
    author_id       TEXT NOT NULL,
    ip_addr         TEXT NOT NULL,
    authed_token_id UUID NOT NULL,
    board_id        UUID NOT NULL,
    thread_id       UUID NOT NULL,
    is_abone        BOOLEAN NOT NULL DEFAULT FALSE,
    res_order       INTEGER NOT NULL,
    client_info     JSONB NOT NULL,
    FOREIGN KEY (board_id) REFERENCES boards (id),
    FOREIGN KEY (thread_id) REFERENCES threads (id) ON DELETE CASCADE
);
CREATE INDEX ON responses (thread_id);

CREATE TABLE caps (
    id              UUID PRIMARY KEY,
    name            VARCHAR(255) NOT NULL,
    description     TEXT NOT NULL,
    password_hash   VARCHAR(255) NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL,
    updated_at      TIMESTAMPTZ NOT NULL
);

CREATE TABLE boards_caps (
    id          UUID PRIMARY KEY,
    board_id    UUID NOT NULL,
    cap_id      UUID NOT NULL,
    FOREIGN KEY (board_id) REFERENCES boards (id),
    FOREIGN KEY (cap_id) REFERENCES caps (id)
);

CREATE TABLE ng_words (
    id          UUID PRIMARY KEY,
    name        VARCHAR(255) NOT NULL,
    word        VARCHAR(255) NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL,
    updated_at  TIMESTAMPTZ NOT NULL
);

CREATE TABLE boards_ng_words (
    id          UUID PRIMARY KEY,
    board_id    UUID NOT NULL,
    ng_word_id  UUID NOT NULL,
    FOREIGN KEY (board_id) REFERENCES boards (id),
    FOREIGN KEY (ng_word_id) REFERENCES ng_words (id)
);

CREATE TABLE admin_roles (
    id              UUID PRIMARY KEY,
    role_name       VARCHAR(255) NOT NULL,
    role_description TEXT NOT NULL
);

CREATE TABLE admin_role_scopes (
    id          UUID PRIMARY KEY,
    role_id     UUID NOT NULL,
    scope_key   VARCHAR(255) NOT NULL,
    FOREIGN KEY (role_id) REFERENCES admin_roles (id)
);
CREATE INDEX ON admin_role_scopes (role_id);

CREATE TABLE admin_users (
    id              UUID PRIMARY KEY,
    user_role_id    UUID NOT NULL,
    FOREIGN KEY (user_role_id) REFERENCES admin_roles (id)
);

-- archived_threads: RANGE partitioned by year of last_modified_at
-- MySQL used PARTITION BY RANGE (YEAR(last_modified_at)); PostgreSQL equivalent below.
CREATE TABLE archived_threads (
    id                      UUID NOT NULL,
    board_id                UUID NOT NULL,
    thread_number           BIGINT NOT NULL,
    last_modified_at        TIMESTAMPTZ NOT NULL,
    sage_last_modified_at   TIMESTAMPTZ NOT NULL,
    title                   TEXT NOT NULL,
    authed_token_id         UUID NOT NULL,
    metadent                TEXT NOT NULL,
    response_count          INT NOT NULL,
    no_pool                 BOOLEAN NOT NULL DEFAULT FALSE,
    active                  BOOLEAN NOT NULL DEFAULT TRUE,
    archived                BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (id, last_modified_at)
) PARTITION BY RANGE (last_modified_at);

CREATE TABLE archived_threads_2024
    PARTITION OF archived_threads
    FOR VALUES FROM ('2024-01-01 00:00:00+00') TO ('2025-01-01 00:00:00+00');
CREATE TABLE archived_threads_2025
    PARTITION OF archived_threads
    FOR VALUES FROM ('2025-01-01 00:00:00+00') TO ('2026-01-01 00:00:00+00');
CREATE TABLE archived_threads_2026
    PARTITION OF archived_threads
    FOR VALUES FROM ('2026-01-01 00:00:00+00') TO ('2027-01-01 00:00:00+00');
CREATE TABLE archived_threads_2027
    PARTITION OF archived_threads
    FOR VALUES FROM ('2027-01-01 00:00:00+00') TO ('2028-01-01 00:00:00+00');
CREATE TABLE archived_threads_2028
    PARTITION OF archived_threads
    FOR VALUES FROM ('2028-01-01 00:00:00+00') TO ('2029-01-01 00:00:00+00');
CREATE TABLE archived_threads_2029
    PARTITION OF archived_threads
    FOR VALUES FROM ('2029-01-01 00:00:00+00') TO ('2030-01-01 00:00:00+00');
CREATE TABLE archived_threads_2030
    PARTITION OF archived_threads
    FOR VALUES FROM ('2030-01-01 00:00:00+00') TO ('2031-01-01 00:00:00+00');
CREATE TABLE archived_threads_default
    PARTITION OF archived_threads DEFAULT;

CREATE INDEX ON archived_threads (thread_number);

-- archived_responses: MySQL used RANGE(MONTH) + HASH(YEAR) subpartitions (100 subpartitions).
-- PostgreSQL does not support HASH subpartitions. Redesigned as RANGE by year.
-- This provides the same data isolation benefit with simpler DDL.
CREATE TABLE archived_responses (
    id              UUID NOT NULL,
    author_name     TEXT NOT NULL,
    mail            TEXT NOT NULL,
    body            TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL,
    author_id       TEXT NOT NULL,
    ip_addr         TEXT NOT NULL,
    authed_token_id UUID NOT NULL,
    board_id        UUID NOT NULL,
    thread_id       UUID NOT NULL,
    is_abone        BOOLEAN NOT NULL DEFAULT FALSE,
    res_order       INTEGER NOT NULL,
    client_info     JSONB NOT NULL,
    PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (created_at);

CREATE TABLE archived_responses_2024
    PARTITION OF archived_responses
    FOR VALUES FROM ('2024-01-01 00:00:00+00') TO ('2025-01-01 00:00:00+00');
CREATE TABLE archived_responses_2025
    PARTITION OF archived_responses
    FOR VALUES FROM ('2025-01-01 00:00:00+00') TO ('2026-01-01 00:00:00+00');
CREATE TABLE archived_responses_2026
    PARTITION OF archived_responses
    FOR VALUES FROM ('2026-01-01 00:00:00+00') TO ('2027-01-01 00:00:00+00');
CREATE TABLE archived_responses_2027
    PARTITION OF archived_responses
    FOR VALUES FROM ('2027-01-01 00:00:00+00') TO ('2028-01-01 00:00:00+00');
CREATE TABLE archived_responses_2028
    PARTITION OF archived_responses
    FOR VALUES FROM ('2028-01-01 00:00:00+00') TO ('2029-01-01 00:00:00+00');
CREATE TABLE archived_responses_2029
    PARTITION OF archived_responses
    FOR VALUES FROM ('2029-01-01 00:00:00+00') TO ('2030-01-01 00:00:00+00');
CREATE TABLE archived_responses_2030
    PARTITION OF archived_responses
    FOR VALUES FROM ('2030-01-01 00:00:00+00') TO ('2031-01-01 00:00:00+00');
CREATE TABLE archived_responses_default
    PARTITION OF archived_responses DEFAULT;

CREATE INDEX ON archived_responses (thread_id);

-- Seed data (UUID literals used directly instead of UUID_TO_BIN)
INSERT INTO boards (id, name, board_key, default_name)
VALUES ('01815522-2d2b-728f-af94-a234aabb6b20'::uuid, '試験板', 'experiment', 'ポッドの名無し');

INSERT INTO boards_info (id, local_rules, created_at, updated_at)
VALUES ('01815522-2d2b-728f-af94-a234aabb6b20'::uuid, '利用規約に従う', NOW(), NOW());
