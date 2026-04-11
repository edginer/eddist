-- PostgreSQL equivalent
-- BINARY(16) → UUID, DATETIME(3) → TIMESTAMPTZ, inline INDEX → separate CREATE INDEX

CREATE TABLE IF NOT EXISTS notices (
    id              UUID PRIMARY KEY,
    slug            VARCHAR(255) NOT NULL UNIQUE,
    title           VARCHAR(255) NOT NULL,
    content         TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL,
    updated_at      TIMESTAMPTZ NOT NULL,
    published_at    TIMESTAMPTZ NOT NULL,
    author_email    VARCHAR(255)
);
CREATE INDEX ON notices (slug);
CREATE INDEX ON notices (published_at);
CREATE INDEX ON notices (author_email);
