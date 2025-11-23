-- Add up migration script here
CREATE TABLE IF NOT EXISTS
    notices (
        id BINARY(16) PRIMARY KEY,
        slug VARCHAR(255) NOT NULL UNIQUE,
        title VARCHAR(255) NOT NULL,
        content TEXT NOT NULL,
        created_at DATETIME(3) NOT NULL,
        updated_at DATETIME(3) NOT NULL,
        published_at DATETIME(3) NOT NULL,
        author_email VARCHAR(255),
        INDEX (slug),
        INDEX (published_at),
        INDEX (author_email)
    );
