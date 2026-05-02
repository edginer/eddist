-- Consolidated PostgreSQL initial schema for eddist.
-- All MySQL-specific constructs replaced:
--   BINARY(16)       → UUID
--   DATETIME(3)      → TIMESTAMPTZ
--   AUTO_INCREMENT   → (not needed; UUIDs used for PKs)
--   Inline INDEX     → separate CREATE INDEX statements
--   JSON             → JSONB
--   ENUM inline      → CREATE TYPE ... AS ENUM
--   ON DUPLICATE KEY → ON CONFLICT ... DO UPDATE / DO NOTHING

-- Extensions
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- authed_tokens (columns from all migrations baked in)
CREATE TABLE authed_tokens (
    id                        UUID PRIMARY KEY,
    token                     VARCHAR(255) UNIQUE NOT NULL,
    origin_ip                 VARCHAR(255) NOT NULL,
    reduced_origin_ip         VARCHAR(255) NOT NULL,
    writing_ua                TEXT NOT NULL,
    authed_ua                 TEXT,
    auth_code                 VARCHAR(12) NOT NULL,
    created_at                TIMESTAMPTZ NOT NULL,
    authed_at                 TIMESTAMPTZ,
    validity                  BOOLEAN NOT NULL,
    last_wrote_at             TIMESTAMPTZ,
    author_id_seed            BYTEA NOT NULL DEFAULT '\x00'::bytea,
    require_user_registration BOOLEAN NOT NULL DEFAULT FALSE,
    registered_user_id        UUID,
    asn_num                   INT NOT NULL DEFAULT 0,
    additional_info           JSONB,
    require_reauth            BOOLEAN NOT NULL DEFAULT FALSE
);
CREATE INDEX ON authed_tokens (origin_ip);
CREATE INDEX ON authed_tokens (auth_code);

-- boards
CREATE TABLE boards (
    id           UUID PRIMARY KEY,
    name         VARCHAR(255) NOT NULL,
    board_key    VARCHAR(255) UNIQUE NOT NULL,
    default_name TEXT NOT NULL
);

-- boards_info (force_metadent_type column baked in)
CREATE TABLE boards_info (
    id                                   UUID PRIMARY KEY,
    local_rules                          TEXT NOT NULL,
    base_thread_creation_span_sec        INT NOT NULL DEFAULT 120,
    base_response_creation_span_sec      INT NOT NULL DEFAULT 5,
    max_thread_name_byte_length          INT NOT NULL DEFAULT 256,
    max_author_name_byte_length          INT NOT NULL DEFAULT 128,
    max_email_byte_length                INT NOT NULL DEFAULT 128,
    max_response_body_byte_length        INT NOT NULL DEFAULT 9192,
    max_response_body_lines              INT NOT NULL DEFAULT 32,
    threads_archive_cron                 VARCHAR(255),
    threads_archive_trigger_thread_count INT,
    read_only                            BOOLEAN NOT NULL DEFAULT FALSE,
    created_at                           TIMESTAMPTZ NOT NULL,
    updated_at                           TIMESTAMPTZ NOT NULL,
    force_metadent_type                  VARCHAR(10) DEFAULT NULL,
    FOREIGN KEY (id) REFERENCES boards (id)
);

-- threads
CREATE TABLE threads (
    id                    UUID PRIMARY KEY,
    board_id              UUID NOT NULL,
    thread_number         BIGINT NOT NULL,
    last_modified_at      TIMESTAMPTZ NOT NULL,
    sage_last_modified_at TIMESTAMPTZ NOT NULL,
    title                 TEXT NOT NULL,
    authed_token_id       UUID NOT NULL,
    metadent              TEXT NOT NULL,
    response_count        INT NOT NULL,
    no_pool               BOOLEAN NOT NULL DEFAULT FALSE,
    active                BOOLEAN NOT NULL DEFAULT TRUE,
    archived              BOOLEAN NOT NULL DEFAULT FALSE,
    archive_converted     BOOLEAN NOT NULL DEFAULT FALSE,
    FOREIGN KEY (board_id) REFERENCES boards (id),
    FOREIGN KEY (authed_token_id) REFERENCES authed_tokens (id),
    UNIQUE (board_id, thread_number)
);
CREATE INDEX ON threads (thread_number);

-- responses
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
CREATE INDEX idx_res_order_1_thread ON responses (res_order, thread_id);

-- caps
CREATE TABLE caps (
    id            UUID PRIMARY KEY,
    name          VARCHAR(255) NOT NULL,
    description   TEXT NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL,
    updated_at    TIMESTAMPTZ NOT NULL
);

-- boards_caps
CREATE TABLE boards_caps (
    id       UUID PRIMARY KEY,
    board_id UUID NOT NULL,
    cap_id   UUID NOT NULL,
    FOREIGN KEY (board_id) REFERENCES boards (id),
    FOREIGN KEY (cap_id) REFERENCES caps (id)
);

-- ng_words
CREATE TABLE ng_words (
    id         UUID PRIMARY KEY,
    name       VARCHAR(255) NOT NULL,
    word       VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

-- boards_ng_words
CREATE TABLE boards_ng_words (
    id         UUID PRIMARY KEY,
    board_id   UUID NOT NULL,
    ng_word_id UUID NOT NULL,
    FOREIGN KEY (board_id) REFERENCES boards (id),
    FOREIGN KEY (ng_word_id) REFERENCES ng_words (id)
);

-- admin_roles
CREATE TABLE admin_roles (
    id               UUID PRIMARY KEY,
    role_name        VARCHAR(255) NOT NULL,
    role_description TEXT NOT NULL
);

-- admin_role_scopes
CREATE TABLE admin_role_scopes (
    id        UUID PRIMARY KEY,
    role_id   UUID NOT NULL,
    scope_key VARCHAR(255) NOT NULL,
    FOREIGN KEY (role_id) REFERENCES admin_roles (id)
);
CREATE INDEX ON admin_role_scopes (role_id);

-- admin_users
CREATE TABLE admin_users (
    id           UUID PRIMARY KEY,
    user_role_id UUID NOT NULL,
    FOREIGN KEY (user_role_id) REFERENCES admin_roles (id)
);

-- archived_threads: RANGE partitioned by year of last_modified_at
CREATE TABLE archived_threads (
    id                    UUID NOT NULL,
    board_id              UUID NOT NULL,
    thread_number         BIGINT NOT NULL,
    last_modified_at      TIMESTAMPTZ NOT NULL,
    sage_last_modified_at TIMESTAMPTZ NOT NULL,
    title                 TEXT NOT NULL,
    authed_token_id       UUID NOT NULL,
    metadent              TEXT NOT NULL,
    response_count        INT NOT NULL,
    no_pool               BOOLEAN NOT NULL DEFAULT FALSE,
    active                BOOLEAN NOT NULL DEFAULT TRUE,
    archived              BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (id, last_modified_at)
) PARTITION BY RANGE (last_modified_at);

CREATE TABLE archived_threads_2024 PARTITION OF archived_threads FOR VALUES FROM ('2024-01-01 00:00:00+00') TO ('2025-01-01 00:00:00+00');
CREATE TABLE archived_threads_2025 PARTITION OF archived_threads FOR VALUES FROM ('2025-01-01 00:00:00+00') TO ('2026-01-01 00:00:00+00');
CREATE TABLE archived_threads_2026 PARTITION OF archived_threads FOR VALUES FROM ('2026-01-01 00:00:00+00') TO ('2027-01-01 00:00:00+00');
CREATE TABLE archived_threads_2027 PARTITION OF archived_threads FOR VALUES FROM ('2027-01-01 00:00:00+00') TO ('2028-01-01 00:00:00+00');
CREATE TABLE archived_threads_2028 PARTITION OF archived_threads FOR VALUES FROM ('2028-01-01 00:00:00+00') TO ('2029-01-01 00:00:00+00');
CREATE TABLE archived_threads_2029 PARTITION OF archived_threads FOR VALUES FROM ('2029-01-01 00:00:00+00') TO ('2030-01-01 00:00:00+00');
CREATE TABLE archived_threads_2030 PARTITION OF archived_threads FOR VALUES FROM ('2030-01-01 00:00:00+00') TO ('2031-01-01 00:00:00+00');
CREATE TABLE archived_threads_2031 PARTITION OF archived_threads FOR VALUES FROM ('2031-01-01 00:00:00+00') TO ('2032-01-01 00:00:00+00');
CREATE TABLE archived_threads_2032 PARTITION OF archived_threads FOR VALUES FROM ('2032-01-01 00:00:00+00') TO ('2033-01-01 00:00:00+00');
CREATE TABLE archived_threads_2033 PARTITION OF archived_threads FOR VALUES FROM ('2033-01-01 00:00:00+00') TO ('2034-01-01 00:00:00+00');
CREATE TABLE archived_threads_2034 PARTITION OF archived_threads FOR VALUES FROM ('2034-01-01 00:00:00+00') TO ('2035-01-01 00:00:00+00');
CREATE TABLE archived_threads_2035 PARTITION OF archived_threads FOR VALUES FROM ('2035-01-01 00:00:00+00') TO ('2036-01-01 00:00:00+00');
CREATE TABLE archived_threads_2036 PARTITION OF archived_threads FOR VALUES FROM ('2036-01-01 00:00:00+00') TO ('2037-01-01 00:00:00+00');
CREATE TABLE archived_threads_default PARTITION OF archived_threads DEFAULT;
CREATE INDEX ON archived_threads (thread_number);

-- archived_responses: RANGE partitioned by year of created_at
-- MySQL used RANGE(MONTH) + HASH(YEAR) subpartitions; PostgreSQL redesigned as RANGE-by-year.
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

CREATE TABLE archived_responses_2024 PARTITION OF archived_responses FOR VALUES FROM ('2024-01-01 00:00:00+00') TO ('2025-01-01 00:00:00+00');
CREATE TABLE archived_responses_2025 PARTITION OF archived_responses FOR VALUES FROM ('2025-01-01 00:00:00+00') TO ('2026-01-01 00:00:00+00');
CREATE TABLE archived_responses_2026 PARTITION OF archived_responses FOR VALUES FROM ('2026-01-01 00:00:00+00') TO ('2027-01-01 00:00:00+00');
CREATE TABLE archived_responses_2027 PARTITION OF archived_responses FOR VALUES FROM ('2027-01-01 00:00:00+00') TO ('2028-01-01 00:00:00+00');
CREATE TABLE archived_responses_2028 PARTITION OF archived_responses FOR VALUES FROM ('2028-01-01 00:00:00+00') TO ('2029-01-01 00:00:00+00');
CREATE TABLE archived_responses_2029 PARTITION OF archived_responses FOR VALUES FROM ('2029-01-01 00:00:00+00') TO ('2030-01-01 00:00:00+00');
CREATE TABLE archived_responses_2030 PARTITION OF archived_responses FOR VALUES FROM ('2030-01-01 00:00:00+00') TO ('2031-01-01 00:00:00+00');
CREATE TABLE archived_responses_2031 PARTITION OF archived_responses FOR VALUES FROM ('2031-01-01 00:00:00+00') TO ('2032-01-01 00:00:00+00');
CREATE TABLE archived_responses_2032 PARTITION OF archived_responses FOR VALUES FROM ('2032-01-01 00:00:00+00') TO ('2033-01-01 00:00:00+00');
CREATE TABLE archived_responses_2033 PARTITION OF archived_responses FOR VALUES FROM ('2033-01-01 00:00:00+00') TO ('2034-01-01 00:00:00+00');
CREATE TABLE archived_responses_2034 PARTITION OF archived_responses FOR VALUES FROM ('2034-01-01 00:00:00+00') TO ('2035-01-01 00:00:00+00');
CREATE TABLE archived_responses_2035 PARTITION OF archived_responses FOR VALUES FROM ('2035-01-01 00:00:00+00') TO ('2036-01-01 00:00:00+00');
CREATE TABLE archived_responses_2036 PARTITION OF archived_responses FOR VALUES FROM ('2036-01-01 00:00:00+00') TO ('2037-01-01 00:00:00+00');
CREATE TABLE archived_responses_default PARTITION OF archived_responses DEFAULT;
CREATE INDEX ON archived_responses (thread_id);

-- idps
CREATE TABLE idps (
    id               UUID PRIMARY KEY,
    idp_name         VARCHAR(255) NOT NULL,
    idp_display_name VARCHAR(255) NOT NULL,
    idp_logo_svg     TEXT,
    oidc_config_url  TEXT NOT NULL,
    client_id        TEXT NOT NULL,
    client_secret    TEXT NOT NULL,
    enabled          BOOLEAN NOT NULL
);

-- users
CREATE TABLE users (
    id         UUID PRIMARY KEY,
    user_name  VARCHAR(255) NOT NULL,
    enabled    BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

-- user_idp_bindings
CREATE TABLE user_idp_bindings (
    id         UUID PRIMARY KEY,
    user_id    UUID NOT NULL,
    idp_id     UUID NOT NULL,
    idp_sub    VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users (id),
    FOREIGN KEY (idp_id) REFERENCES idps (id),
    UNIQUE (user_id, idp_id)
);
CREATE INDEX ON user_idp_bindings (idp_id);
CREATE INDEX ON user_idp_bindings (user_id);

-- user_authed_tokens
CREATE TABLE user_authed_tokens (
    id              UUID PRIMARY KEY,
    user_id         UUID NOT NULL,
    authed_token_id UUID NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL,
    updated_at      TIMESTAMPTZ NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users (id),
    FOREIGN KEY (authed_token_id) REFERENCES authed_tokens (id),
    UNIQUE (user_id, authed_token_id)
);
CREATE INDEX ON user_authed_tokens (authed_token_id);
CREATE INDEX ON user_authed_tokens (user_id);

-- FK from authed_tokens to users (added after users table is defined above)
ALTER TABLE authed_tokens ADD CONSTRAINT authed_tokens_registered_user_id_foreign
    FOREIGN KEY (registered_user_id) REFERENCES users (id);

-- user_restriction_rules
CREATE TYPE restriction_rule_type AS ENUM ('ASN', 'IP', 'IP_CIDR', 'USER_AGENT');
CREATE TABLE user_restriction_rules (
    id               UUID PRIMARY KEY,
    name             VARCHAR(255) NOT NULL,
    rule_type        restriction_rule_type NOT NULL,
    rule_value       TEXT NOT NULL,
    expires_at       TIMESTAMPTZ,
    created_at       TIMESTAMPTZ NOT NULL,
    updated_at       TIMESTAMPTZ NOT NULL,
    created_by_email VARCHAR(255) NOT NULL
);
CREATE INDEX ON user_restriction_rules (rule_type);
CREATE INDEX ON user_restriction_rules (expires_at);
CREATE INDEX ON user_restriction_rules (created_at);

-- notices
CREATE TABLE notices (
    id           UUID PRIMARY KEY,
    slug         VARCHAR(255) NOT NULL UNIQUE,
    title        VARCHAR(255) NOT NULL,
    content      TEXT NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL,
    updated_at   TIMESTAMPTZ NOT NULL,
    published_at TIMESTAMPTZ NOT NULL,
    author_email VARCHAR(255)
);
CREATE INDEX ON notices (slug);
CREATE INDEX ON notices (published_at);
CREATE INDEX ON notices (author_email);

-- terms
CREATE TABLE terms (
    id         UUID PRIMARY KEY,
    content    TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    updated_by VARCHAR(255)
);
CREATE INDEX ON terms (updated_at);

-- captcha_configs (endpoint_usage column baked in)
CREATE TABLE captcha_configs (
    id                     UUID PRIMARY KEY,
    name                   VARCHAR(100) NOT NULL,
    provider               VARCHAR(50) NOT NULL,
    site_key               TEXT NOT NULL,
    secret                 TEXT NOT NULL,
    base_url               VARCHAR(255),
    widget_form_field_name VARCHAR(100),
    widget_script_url      TEXT,
    widget_html            TEXT,
    widget_script_handler  TEXT,
    capture_fields         JSONB,
    verification           JSONB,
    is_active              BOOLEAN NOT NULL DEFAULT TRUE,
    display_order          INT NOT NULL DEFAULT 0,
    created_at             TIMESTAMPTZ NOT NULL,
    updated_at             TIMESTAMPTZ NOT NULL,
    updated_by             VARCHAR(255),
    endpoint_usage         VARCHAR(16) NOT NULL DEFAULT 'auth_code'
);
CREATE INDEX idx_captcha_configs_provider ON captcha_configs (provider);
CREATE INDEX idx_captcha_configs_is_active ON captcha_configs (is_active);
CREATE INDEX idx_captcha_configs_display_order ON captcha_configs (display_order);

-- server_settings
CREATE TABLE server_settings (
    id          UUID PRIMARY KEY,
    setting_key VARCHAR(255) NOT NULL UNIQUE,
    value       TEXT NOT NULL,
    description TEXT,
    created_at  TIMESTAMPTZ NOT NULL,
    updated_at  TIMESTAMPTZ NOT NULL
);
CREATE INDEX idx_server_settings_key ON server_settings (setting_key);

-- Seed data
INSERT INTO boards (id, name, board_key, default_name)
VALUES ('01815522-2d2b-728f-af94-a234aabb6b20'::uuid, '試験板', 'experiment', 'ポッドの名無し');

INSERT INTO boards_info (id, local_rules, created_at, updated_at)
VALUES ('01815522-2d2b-728f-af94-a234aabb6b20'::uuid, '利用規約に従う', NOW(), NOW());

INSERT INTO terms (id, content, created_at, updated_at, updated_by)
VALUES (
    '019af3e1-2caa-788c-98da-b4d0a5e1c25a'::uuid,
    '## 第1条（適用範囲）
本利用規約（以下「本規約」といいます）は、当掲示板（以下「本サービス」といいます）を利用するすべてのユーザー（以下「利用者」といいます）に適用されます。利用者は、本サービスを利用することにより、本規約に同意したものとみなされます。

## 第2条（収集する情報）
本サービスは、利用者のIPアドレス、Cookie、その他端末を特定するための情報を収集し、以下の目的で使用します。

- 本サービスの運営及び管理
- 不正利用の防止及びセキュリティの向上
- サービスの改善及び提供内容の最適化

これらの情報は、本サービス運営のためにのみ利用され、以下の場合に加えて法執行機関等からの正当な要求に応じる場合、または利用者が同意した場合を除き、第三者に提供することはありません。

- 書き込み時、また書き込み前の認証時に利用者の正当性を確認するために、いくつかのサービス(*1)に問い合わせる場合

## 第3条（書き込みの責任）
本サービスにおけるすべての書き込み（テキスト、画像、その他の情報を含む）は、その書き込みを行った利用者に全責任が属します。利用者は、以下に定める違法な書き込みや不適切な内容を投稿しないことに同意するものとします。

## 第4条（禁止事項）
利用者は、以下の行為を行ってはなりません。

### 違法な書き込み

- 名誉毀損、中傷、侮辱、脅迫など、他者の権利や名誉を侵害する内容
- 著作権、商標権、特許権、プライバシー権、肖像権などの知的財産権を侵害する内容
- 無断で個人情報（氏名、住所、電話番号、メールアドレスなど）を公開する行為
- 法律で禁止されている行為を助長する内容や、犯罪行為に関与する内容

### その他不適切な書き込み

- 過度に暴力的な表現、残虐な表現、児童ポルノを含む内容
- 虚偽の情報を流布し、混乱や誤解を招く内容
- スパム、商業目的の宣伝、不正アクセス行為に関与する内容
- スクリプトやbotなどを用いた自動書き込み行為
- 人種、民族、国籍、性別、宗教、障害、性的指向などに対する差別的な発言

## 第5条（著作権）
利用者が本サービスに投稿した書き込みの著作権は、書き込みを行った利用者自身に属します。ただし、利用者は本サービス及び本サービスの関連サービス(*2)で投稿内容を使用、複製、編集、公開することについて、運営者に対して無期限かつ無償で非独占的な使用権を付与し、著作者人格権を行使しないことに同意します。利用者は、利用者自身の書き込みが第三者によって無断で転載されることを防止するため、本サービスに書き込みを行う際には原則、本サービスならびに本サービスに関連するサービス以外への転載を許諾しないものとして書き込むことに同意します。

## 第6条（違反行為への対応）
本サービスの運営側は、利用者の書き込み内容が本規約に違反している、または不適切であると判断した場合、当該書き込みを事前通知なく削除する権利を有します。また、法執行機関や、名誉毀損や中傷に関する被害者からの正当な求めがあった場合、投稿内容の削除および発信者情報の開示に応じることがあります。また、違反行為を繰り返す利用者に対してはアカウントの一時停止などの措置を取ることがあります。

## 第7条（免責事項）
本サービスは、利用者が本サービスの利用に関連して被ったあらゆる損害等について、一切の責任を負いません。利用者は、自己の責任で本サービスを利用するものとし、運営側に対して一切の賠償請求を行わないものとします。

## 第8条（規約の改定）
本規約は、必要に応じて改定されることがあります。改定後の規約は、本サービス上に掲載された時点で効力を発生します。利用者は、定期的に本規約を確認する義務を負い、改定後も本サービスの利用を継続した場合、改定内容に同意したものとみなされます。

---

1. 例: hCaptcha, Cloudflare Turnstile, Spur
2. 本サービスの運営者、もしくは運営者が委託する第三者が運営するサービス、加えていずれの場合も本サービスが使用するドメインを含むサービス
',
    NOW(),
    NOW(),
    NULL
);
