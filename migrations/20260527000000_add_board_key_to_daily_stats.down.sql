ALTER TABLE daily_stats
    DROP PRIMARY KEY,
    DROP COLUMN board_key,
    ADD PRIMARY KEY (date);
