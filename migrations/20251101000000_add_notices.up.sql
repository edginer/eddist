-- Add up migration script here
CREATE TABLE IF NOT EXISTS
    notices (
        id BINARY(16) PRIMARY KEY,
        title VARCHAR(255) NOT NULL,
        content TEXT NOT NULL,
        summary TEXT,
        created_at DATETIME(3) NOT NULL,
        updated_at DATETIME(3) NOT NULL,
        published_at DATETIME(3) NOT NULL,
        author_id BINARY(16),
        FOREIGN KEY (author_id) REFERENCES admin_users (id),
        INDEX (published_at),
        INDEX (author_id)
    );
