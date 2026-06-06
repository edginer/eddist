use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::{
    auth::AdminIdentity,
    models::{Res, Thread},
    repository::{
        admin_archive_repository::{
            AdminArchiveRepository, ArchivedAdminThread, ArchivedResUpdate, ArchivedThread,
        },
        admin_response_repository::AdminResponseRepository,
        admin_thread_repository::AdminThreadRepository,
    },
};

#[async_trait::async_trait]
pub trait AdminArchiveService: Send + Sync {
    async fn get_archived_threads(
        &self,
        board_key: &str,
        keyword: Option<&str>,
        date_range: (Option<DateTime<Utc>>, Option<DateTime<Utc>>),
        page: u64,
        limit: u64,
    ) -> anyhow::Result<Vec<Thread>>;
    async fn get_archived_thread(
        &self,
        board_key: &str,
        thread_id: u64,
    ) -> anyhow::Result<Option<Thread>>;
    async fn get_archived_responses(
        &self,
        board_key: &str,
        thread_id: u64,
    ) -> anyhow::Result<Vec<Res>>;
    async fn get_dat_archived_thread(
        &self,
        board_key: &str,
        thread_number: u64,
    ) -> anyhow::Result<ArchivedThread>;
    async fn get_admin_dat_archived_thread(
        &self,
        board_key: &str,
        thread_number: u64,
    ) -> anyhow::Result<ArchivedAdminThread>;
    async fn update_archived_res(
        &self,
        actor: &AdminIdentity,
        board_key: &str,
        thread_number: u64,
        updates: &[ArchivedResUpdate],
    ) -> anyhow::Result<()>;
    async fn delete_archived_res(
        &self,
        actor: &AdminIdentity,
        board_key: &str,
        thread_number: u64,
        res_order: u64,
    ) -> anyhow::Result<()>;
    async fn delete_archived_thread(
        &self,
        actor: &AdminIdentity,
        board_key: &str,
        thread_number: u64,
    ) -> anyhow::Result<()>;
}

pub struct ArchiveServiceImpl {
    thread_repo: Arc<dyn AdminThreadRepository>,
    response_repo: Arc<dyn AdminResponseRepository>,
    archive_repo: Arc<dyn AdminArchiveRepository>,
}

impl ArchiveServiceImpl {
    pub fn new(
        thread_repo: Arc<dyn AdminThreadRepository>,
        response_repo: Arc<dyn AdminResponseRepository>,
        archive_repo: Arc<dyn AdminArchiveRepository>,
    ) -> Self {
        Self {
            thread_repo,
            response_repo,
            archive_repo,
        }
    }
}

#[async_trait::async_trait]
impl AdminArchiveService for ArchiveServiceImpl {
    async fn get_archived_threads(
        &self,
        board_key: &str,
        keyword: Option<&str>,
        date_range: (Option<DateTime<Utc>>, Option<DateTime<Utc>>),
        page: u64,
        limit: u64,
    ) -> anyhow::Result<Vec<Thread>> {
        self.thread_repo
            .get_archived_threads_by_filter(board_key, keyword, date_range, page, limit)
            .await
    }

    async fn get_archived_thread(
        &self,
        board_key: &str,
        thread_id: u64,
    ) -> anyhow::Result<Option<Thread>> {
        let threads = self
            .thread_repo
            .get_archived_threads_by_thread_id(board_key, Some(vec![thread_id]))
            .await?;
        Ok(threads.into_iter().next())
    }

    async fn get_archived_responses(
        &self,
        board_key: &str,
        thread_id: u64,
    ) -> anyhow::Result<Vec<Res>> {
        self.response_repo
            .get_archived_reses_by_thread_id(board_key, thread_id)
            .await
    }

    async fn get_dat_archived_thread(
        &self,
        board_key: &str,
        thread_number: u64,
    ) -> anyhow::Result<ArchivedThread> {
        self.archive_repo.get_thread(board_key, thread_number).await
    }

    async fn get_admin_dat_archived_thread(
        &self,
        board_key: &str,
        thread_number: u64,
    ) -> anyhow::Result<ArchivedAdminThread> {
        self.archive_repo
            .get_archived_admin_thread(board_key, thread_number)
            .await
    }

    async fn update_archived_res(
        &self,
        _actor: &AdminIdentity,
        board_key: &str,
        thread_number: u64,
        updates: &[ArchivedResUpdate],
    ) -> anyhow::Result<()> {
        self.archive_repo
            .update_response(board_key, thread_number, updates)
            .await
    }

    async fn delete_archived_res(
        &self,
        _actor: &AdminIdentity,
        board_key: &str,
        thread_number: u64,
        res_order: u64,
    ) -> anyhow::Result<()> {
        self.archive_repo
            .delete_response(board_key, thread_number, res_order)
            .await
    }

    async fn delete_archived_thread(
        &self,
        _actor: &AdminIdentity,
        board_key: &str,
        thread_number: u64,
    ) -> anyhow::Result<()> {
        self.archive_repo
            .delete_thread(board_key, thread_number)
            .await
    }
}
