ALTER TABLE daily_stats
    ADD COLUMN board_key VARCHAR(255) NOT NULL DEFAULT 'liveedge',
    DROP PRIMARY KEY,
    ADD PRIMARY KEY (date, board_key);
