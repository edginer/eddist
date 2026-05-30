use std::sync::Arc;

use uuid::Uuid;

use crate::{
    auth::AdminIdentity,
    models::{Res, Thread, UpdateResInput},
    repository::{
        admin_response_repository::AdminResponseRepository,
        admin_thread_repository::AdminThreadRepository,
    },
};

#[async_trait::async_trait]
pub trait ThreadService: Send + Sync {
    async fn get_threads(&self, board_key: &str) -> anyhow::Result<Vec<Thread>>;
    async fn get_thread(&self, board_key: &str, thread_id: u64) -> anyhow::Result<Option<Thread>>;
    async fn get_responses(&self, board_key: &str, thread_id: u64) -> anyhow::Result<Vec<Res>>;
    async fn update_response(
        &self,
        actor: &AdminIdentity,
        board_key: &str,
        thread_id: u64,
        res_id: Uuid,
        input: UpdateResInput,
    ) -> anyhow::Result<Res>;
    async fn compact_threads(
        &self,
        actor: &AdminIdentity,
        board_key: &str,
        target_count: u32,
    ) -> anyhow::Result<()>;
}

pub struct ThreadServiceImpl {
    thread_repo: Arc<dyn AdminThreadRepository>,
    response_repo: Arc<dyn AdminResponseRepository>,
    redis_conn: redis::aio::ConnectionManager,
}

impl ThreadServiceImpl {
    pub fn new(
        thread_repo: Arc<dyn AdminThreadRepository>,
        response_repo: Arc<dyn AdminResponseRepository>,
        redis_conn: redis::aio::ConnectionManager,
    ) -> Self {
        Self {
            thread_repo,
            response_repo,
            redis_conn,
        }
    }
}

#[async_trait::async_trait]
impl ThreadService for ThreadServiceImpl {
    async fn get_threads(&self, board_key: &str) -> anyhow::Result<Vec<Thread>> {
        self.thread_repo
            .get_threads_by_thread_id(board_key, None)
            .await
    }

    async fn get_thread(&self, board_key: &str, thread_id: u64) -> anyhow::Result<Option<Thread>> {
        let threads = self
            .thread_repo
            .get_threads_by_thread_id(board_key, Some(vec![thread_id]))
            .await?;
        Ok(threads.into_iter().next())
    }

    async fn get_responses(&self, board_key: &str, thread_id: u64) -> anyhow::Result<Vec<Res>> {
        self.response_repo
            .get_reses_by_thread_id(board_key, thread_id)
            .await
    }

    async fn update_response(
        &self,
        _actor: &AdminIdentity,
        board_key: &str,
        _thread_id: u64,
        res_id: Uuid,
        input: UpdateResInput,
    ) -> anyhow::Result<Res> {
        use eddist_core::domain::res::ResView;

        let (res, default_name, board_key_actual, thread_number, thread_title) =
            self.response_repo.get_res(res_id).await?;

        let updated_res = self
            .response_repo
            .update_res(
                res_id,
                input.author_name.clone(),
                input.mail.clone(),
                input.body.clone(),
                input.is_abone,
            )
            .await?;

        let author_name = input
            .author_name
            .unwrap_or_else(|| res.author_name.unwrap_or(default_name.clone()));
        let mail = input.mail.unwrap_or_default();
        let is_abone = input.is_abone.unwrap_or(res.is_abone);
        let body = input.body.unwrap_or(res.body);

        let res_view = ResView {
            author_name,
            mail,
            body,
            created_at: res.created_at,
            author_id: res.author_id,
            is_abone,
        };
        let res_view = res_view.get_sjis_bytes(&default_name, thread_title.as_deref());

        let _ = board_key; // use the board_key from the actual DB record
        let mut conn = self.redis_conn.clone();
        let _ = conn
            .send_packed_command(&redis::Cmd::lset(
                format!("threads:{}:{}", board_key_actual, thread_number),
                res.res_order as isize - 1,
                res_view.get_inner(),
            ))
            .await;

        Ok(updated_res)
    }

    async fn compact_threads(
        &self,
        _actor: &AdminIdentity,
        board_key: &str,
        target_count: u32,
    ) -> anyhow::Result<()> {
        self.thread_repo
            .compact_threads(board_key, target_count)
            .await
    }
}
