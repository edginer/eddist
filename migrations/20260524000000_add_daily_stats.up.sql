CREATE TABLE daily_stats (
    date            DATE         NOT NULL PRIMARY KEY,
    total_responses BIGINT       NOT NULL DEFAULT 0,
    new_threads     BIGINT       NOT NULL DEFAULT 0,
    created_at      DATETIME(3)  NOT NULL DEFAULT CURRENT_TIMESTAMP(3)
);
