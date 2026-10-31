use std::sync::Arc;

use eddist_core::domain::{board::BoardInfo, res::get_1001_sjis_bytes};
use metrics::counter;

use crate::{
    domain::{thread::Thread, thread_res_list::ThreadResList},
    repositories::bbs_repository::BbsRepository,
    stores::thread_dat_cache::ThreadDatCache,
};

use super::AppService;

/// The cache and DB paths must append these exact bytes.
fn stopper_message(board_info: &BoardInfo, thread: &Thread) -> Option<Vec<u8>> {
    board_info.enable_1001_message.then(|| {
        get_1001_sjis_bytes(
            thread.thread_number,
            thread.last_modified_at,
            board_info.custom_1001_message.as_deref(),
        )
        .get_inner()
    })
}

#[derive(Clone)]
pub struct ThreadRetrievalService<T: BbsRepository> {
    repo: T,
    dat_cache: Arc<dyn ThreadDatCache>,
}

impl<T: BbsRepository> ThreadRetrievalService<T> {
    pub fn new(repo: T, dat_cache: Arc<dyn ThreadDatCache>) -> Self {
        Self { repo, dat_cache }
    }

    async fn compute_stopper_bytes(&self, board_key: &str, thread_number: u64) -> Option<Vec<u8>> {
        let board = self.repo.get_board(board_key).await.ok()??;
        let board_info = self.repo.get_board_info(board.id).await.ok()??;
        if !board_info.enable_1001_message {
            return None;
        }
        let thread = self
            .repo
            .get_thread_by_board_key_and_thread_number(board_key, thread_number)
            .await
            .ok()??;
        stopper_message(&board_info, &thread)
    }
}

#[async_trait::async_trait]
impl<T: BbsRepository> AppService<ThreadRetrievalServiceInput, DatReadResult, DatRetrievalError>
    for ThreadRetrievalService<T>
{
    async fn execute(
        &self,
        input: ThreadRetrievalServiceInput,
    ) -> Result<DatReadResult, DatRetrievalError> {
        match self
            .dat_cache
            .read(&input.board_key, input.thread_number)
            .await
        {
            Ok(chunks) if !chunks.is_empty() => {
                counter!("dat_retrieval", "source" => "cache").increment(1);

                // Must compute stopper before the 304 check so total_len
                // includes it (the ETag byte-size encodes the full payload).
                let stopper = if chunks.len() >= 1000 {
                    self.compute_stopper_bytes(&input.board_key, input.thread_number)
                        .await
                } else {
                    None
                };

                let raw_len: usize = chunks.iter().map(|c| c.len()).sum();
                let total_len = raw_len + stopper.as_ref().map(|b| b.len()).unwrap_or(0);

                if input.expected_byte_size == Some(total_len) {
                    return Ok(DatReadResult::Unchanged);
                }

                let mut raw = Vec::with_capacity(total_len);
                for chunk in chunks {
                    raw.extend_from_slice(&chunk);
                }
                if let Some(s) = stopper {
                    raw.extend_from_slice(&s);
                }

                Ok(DatReadResult::Found(raw))
            }
            _ => {
                counter!("dat_retrieval", "source" => "db").increment(1);
                let Some(board) = self.repo.get_board(&input.board_key).await? else {
                    return Err(DatRetrievalError::BoardNotFound);
                };
                let board_info = self
                    .repo
                    .get_board_info(board.id)
                    .await?
                    .ok_or(DatRetrievalError::BoardNotFound)?;

                let th = self
                    .repo
                    .get_thread_by_board_key_and_thread_number(
                        &input.board_key,
                        input.thread_number,
                    )
                    .await?;
                let Some(th) = th else {
                    return Err(DatRetrievalError::ThreadNotFound);
                };
                let responses = self.repo.get_responses(th.id).await?;

                let stopper = if th.response_count >= 1000 {
                    stopper_message(&board_info, &th)
                } else {
                    None
                };

                let th_res_list = ThreadResList {
                    thread: th,
                    res_list: responses,
                };
                let mut raw = th_res_list.get_sjis_thread_res_list(&board.default_name);
                if let Some(s) = stopper {
                    raw.extend_from_slice(&s);
                }

                Ok(DatReadResult::Found(raw))
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DatRetrievalError {
    #[error("board not found")]
    BoardNotFound,
    #[error("thread not found")]
    ThreadNotFound,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub struct ThreadRetrievalServiceInput {
    pub board_key: String,
    pub thread_number: u64,
    /// Only checked against the cache, where a match skips materializing the payload.
    pub expected_byte_size: Option<usize>,
}

#[derive(Debug, Clone)]
pub enum DatReadResult {
    Found(Vec<u8>),
    Unchanged,
}
