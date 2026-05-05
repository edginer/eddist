use anyhow::anyhow;
use eddist_core::domain::res::get_1001_sjis_bytes;
use metrics::counter;
use redis::{AsyncCommands, aio::ConnectionManager};

use crate::{domain::thread_res_list::ThreadResList, repositories::bbs_repository::BbsRepository};
use eddist_core::redis_keys::thread_cache_key;

use super::AppService;

#[derive(Clone)]
pub struct ThreadRetrievalService<T: BbsRepository>(T, ConnectionManager);

impl<T: BbsRepository> ThreadRetrievalService<T> {
    pub fn new(repo: T, redis_conn: ConnectionManager) -> Self {
        Self(repo, redis_conn)
    }

    async fn compute_stopper_bytes(&self, board_key: &str, thread_number: u64) -> Option<Vec<u8>> {
        let board = self.0.get_board(board_key).await.ok()??;
        let board_info = self.0.get_board_info(board.id).await.ok()??;
        if !board_info.enable_1001_message {
            return None;
        }
        let th = self
            .0
            .get_thread_by_board_key_and_thread_number(board_key, thread_number)
            .await
            .ok()??;
        Some(
            get_1001_sjis_bytes(
                th.thread_number,
                th.last_modified_at,
                board_info.custom_1001_message.as_deref(),
            )
            .get_inner(),
        )
    }
}

#[async_trait::async_trait]
impl<T: BbsRepository> AppService<ThreadRetrievalServiceInput, ThreadResListRaw>
    for ThreadRetrievalService<T>
{
    async fn execute(
        &self,
        input: ThreadRetrievalServiceInput,
    ) -> anyhow::Result<ThreadResListRaw> {
        let mut redis_conn = self.1.clone();

        match redis_conn
            .lrange::<_, Vec<Vec<u8>>>(
                thread_cache_key(&input.board_key, input.thread_number),
                0,
                -1,
            )
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
                    return Ok(ThreadResListRaw { raw: None });
                }

                let mut raw = Vec::with_capacity(total_len);
                for chunk in chunks {
                    raw.extend_from_slice(&chunk);
                }
                if let Some(s) = stopper {
                    raw.extend_from_slice(&s);
                }

                Ok(ThreadResListRaw { raw: Some(raw) })
            }
            _ => {
                counter!("dat_retrieval", "source" => "db").increment(1);
                let Some(board) = self.0.get_board(&input.board_key).await? else {
                    return Err(anyhow!("failed to find board"));
                };
                let board_info = self
                    .0
                    .get_board_info(board.id)
                    .await?
                    .ok_or_else(|| anyhow!("failed to find board info"))?;

                let th = self
                    .0
                    .get_thread_by_board_key_and_thread_number(
                        &input.board_key,
                        input.thread_number,
                    )
                    .await?;
                let Some(th) = th else {
                    return Err(anyhow!("cannot find such thread"));
                };
                let responses = self.0.get_responses(th.id).await?;

                let response_count = th.response_count;
                let thread_number = th.thread_number;
                let last_modified_at = th.last_modified_at;
                let th_res_list = ThreadResList {
                    thread: th,
                    res_list: responses,
                };
                let mut raw = th_res_list.get_sjis_thread_res_list(&board.default_name);

                if response_count >= 1000 && board_info.enable_1001_message {
                    raw.extend_from_slice(
                        &get_1001_sjis_bytes(
                            thread_number,
                            last_modified_at,
                            board_info.custom_1001_message.as_deref(),
                        )
                        .get_inner(),
                    );
                }

                Ok(ThreadResListRaw { raw: Some(raw) })
            }
        }
    }
}

pub struct ThreadRetrievalServiceInput {
    pub board_key: String,
    pub thread_number: u64,
    /// Byte size extracted from the client's If-None-Match ETag. When the
    /// cache total matches this value the service returns `ThreadResListRaw { raw: None }`
    /// (not-modified signal) without allocating the response body.
    pub expected_byte_size: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct ThreadResListRaw {
    raw: Option<Vec<u8>>,
}

impl ThreadResListRaw {
    pub fn raw(self) -> Option<Vec<u8>> {
        self.raw
    }
}
