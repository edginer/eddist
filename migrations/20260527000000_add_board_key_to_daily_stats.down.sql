CREATE TABLE daily_stats_agg AS
    SELECT date, SUM(total_responses) AS total_responses, SUM(new_threads) AS new_threads
    FROM daily_stats
    GROUP BY date;

TRUNCATE TABLE daily_stats;

ALTER TABLE daily_stats
    DROP PRIMARY KEY,
    DROP COLUMN board_key,
    ADD PRIMARY KEY (date);

INSERT INTO daily_stats (date, total_responses, new_threads)
SELECT date, total_responses, new_threads FROM daily_stats_agg;

DROP TABLE daily_stats_agg;
