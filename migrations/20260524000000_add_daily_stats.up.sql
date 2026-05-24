CREATE TABLE daily_stats (
    id              INT AUTO_INCREMENT PRIMARY KEY,
    date            DATE         NOT NULL,
    total_responses BIGINT       NOT NULL DEFAULT 0,
    new_threads     BIGINT       NOT NULL DEFAULT 0,
    created_at      DATETIME(3)  NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    UNIQUE KEY uq_daily_stats_date (date)
);
