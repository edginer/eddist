use chrono::{DateTime, Datelike, Days, FixedOffset, NaiveDate, Utc};
use eddist_entity::daily_stats;
use sea_orm::{
    ActiveValue::Set,
    ColumnTrait, DatabaseConnection, EntityTrait, FromQueryResult, QueryFilter, QueryOrder,
    QuerySelect, TransactionTrait,
    sea_query::{Expr, ExprTrait, OnConflict},
};

#[derive(Debug, Clone, FromQueryResult)]
pub struct BoardDailyStat {
    pub board_key: String,
    pub date: NaiveDate,
    pub total_responses: i64,
    pub new_threads: i64,
}

#[async_trait::async_trait]
pub trait StatsRepository: Send + Sync + 'static {
    async fn get_today_stats_per_board(&self) -> anyhow::Result<Vec<BoardDailyStat>>;
    async fn get_daily_stats_per_board(&self, days: u32) -> anyhow::Result<Vec<BoardDailyStat>>;
    async fn flush_board_stats(&self, snapshot: &[(String, i64, i64)]) -> anyhow::Result<()>;
}

#[derive(Debug, Clone)]
pub struct StatsRepositoryImpl {
    db: DatabaseConnection,
}

impl StatsRepositoryImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

fn jst_date(now: DateTime<Utc>) -> NaiveDate {
    now.with_timezone(&FixedOffset::east_opt(9 * 60 * 60).unwrap())
        .date_naive()
}

fn history_start_date(today: NaiveDate, days: u32) -> Option<NaiveDate> {
    // MySQL DATE_SUB yields NULL past year zero, which matches no rows. Keep
    // that behavior without overflowing chrono or binding a negative year.
    today
        .checked_sub_days(Days::new(u64::from(days)))
        .filter(|date| date.year() >= 0)
}

fn stats_select() -> sea_orm::Select<daily_stats::Entity> {
    use daily_stats::Column;
    daily_stats::Entity::find().select_only().columns([
        Column::BoardKey,
        Column::Date,
        Column::TotalResponses,
        Column::NewThreads,
    ])
}

fn stats_upsert(
    date: NaiveDate,
    board_key: &str,
    response_delta: i64,
    thread_delta: i64,
) -> sea_orm::Insert<daily_stats::ActiveModel> {
    use daily_stats::Column;
    daily_stats::Entity::insert(daily_stats::ActiveModel {
        date: Set(date),
        board_key: Set(board_key.to_owned()),
        total_responses: Set(response_delta),
        new_threads: Set(thread_delta),
        // Preserve the database's created_at default and leave it untouched on conflict.
        ..Default::default()
    })
    .on_conflict(
        OnConflict::columns([Column::Date, Column::BoardKey])
            .value(
                Column::TotalResponses,
                Expr::col((daily_stats::Entity, Column::TotalResponses)).add(response_delta),
            )
            .value(
                Column::NewThreads,
                Expr::col((daily_stats::Entity, Column::NewThreads)).add(thread_delta),
            )
            .to_owned(),
    )
}

#[async_trait::async_trait]
impl StatsRepository for StatsRepositoryImpl {
    async fn get_today_stats_per_board(&self) -> anyhow::Result<Vec<BoardDailyStat>> {
        Ok(stats_select()
            .filter(daily_stats::Column::Date.eq(jst_date(Utc::now())))
            .into_model::<BoardDailyStat>()
            .all(&self.db)
            .await?)
    }

    async fn get_daily_stats_per_board(&self, days: u32) -> anyhow::Result<Vec<BoardDailyStat>> {
        let today = jst_date(Utc::now());
        let mut query = stats_select().filter(daily_stats::Column::Date.lt(today));
        query = match history_start_date(today, days) {
            Some(start) => query.filter(daily_stats::Column::Date.gte(start)),
            None => query.filter(Expr::val(false)),
        };
        Ok(query
            .order_by_desc(daily_stats::Column::Date)
            .order_by_asc(daily_stats::Column::BoardKey)
            .into_model::<BoardDailyStat>()
            .all(&self.db)
            .await?)
    }

    async fn flush_board_stats(&self, snapshot: &[(String, i64, i64)]) -> anyhow::Result<()> {
        // Use the app clock once per operation. A snapshot crossing JST midnight
        // is attributed to the day the flush began, rather than split per row.
        let today = jst_date(Utc::now());
        let tx = self.db.begin().await?;

        for (board_key, response_delta, thread_delta) in snapshot {
            stats_upsert(today, board_key, *response_delta, *thread_delta)
                .exec_without_returning(&tx)
                .await?;
        }

        tx.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{DbBackend, QueryTrait};

    #[test]
    fn jst_calendar_matches_the_sqlx_midnight_year_and_leap_day_baseline() {
        for (utc, expected) in [
            ("2020-01-01T14:59:59.999999999Z", "2020-01-01"),
            ("2020-01-01T15:00:00Z", "2020-01-02"),
            ("2020-12-31T15:00:00Z", "2021-01-01"),
            ("2020-02-28T15:00:00Z", "2020-02-29"),
        ] {
            let now = DateTime::parse_from_rfc3339(utc)
                .unwrap()
                .with_timezone(&Utc);
            assert_eq!(jst_date(now).to_string(), expected);
        }
    }

    #[test]
    fn history_date_arithmetic_preserves_zero_and_overflow_ranges() {
        let today = NaiveDate::from_ymd_opt(2026, 11, 7).unwrap();
        assert_eq!(history_start_date(today, 0), Some(today));
        assert_eq!(
            history_start_date(today, 2),
            Some(NaiveDate::from_ymd_opt(2026, 11, 5).unwrap())
        );
        assert_eq!(history_start_date(today, 740_000).unwrap().year(), 0);
        assert_eq!(history_start_date(today, 740_500), None);
        assert_eq!(history_start_date(today, u32::MAX), None);
    }

    #[test]
    fn upsert_adds_deltas_in_both_mysql_and_postgres_dialects() {
        let date = NaiveDate::from_ymd_opt(2026, 11, 7).unwrap();
        let query = stats_upsert(date, "board", 3, 1);
        let mysql = query.build(DbBackend::MySql);
        assert_eq!(
            mysql.sql,
            "INSERT INTO `daily_stats` (`date`, `board_key`, `total_responses`, `new_threads`) VALUES (?, ?, ?, ?) ON DUPLICATE KEY UPDATE `total_responses` = `daily_stats`.`total_responses` + ?, `new_threads` = `daily_stats`.`new_threads` + ?"
        );
        let postgres = query.build(DbBackend::Postgres);
        assert_eq!(
            postgres.sql,
            "INSERT INTO \"daily_stats\" (\"date\", \"board_key\", \"total_responses\", \"new_threads\") VALUES ($1, $2, $3, $4) ON CONFLICT (\"date\", \"board_key\") DO UPDATE SET \"total_responses\" = \"daily_stats\".\"total_responses\" + $5, \"new_threads\" = \"daily_stats\".\"new_threads\" + $6"
        );
        for statement in [mysql, postgres] {
            assert_eq!(
                statement.values.unwrap().0,
                vec![
                    date.into(),
                    "board".into(),
                    3_i64.into(),
                    1_i64.into(),
                    3_i64.into(),
                    1_i64.into()
                ]
            );
        }
    }
}
