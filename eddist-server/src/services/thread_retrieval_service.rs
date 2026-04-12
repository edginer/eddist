use anyhow::anyhow;
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
                let total_len: usize = chunks.iter().map(|c| c.len()).sum();
                if input.expected_byte_size == Some(total_len) {
                    return Ok(ThreadResListRaw { raw: None });
                }
                let mut raw = Vec::with_capacity(total_len);
                for chunk in chunks {
                    raw.extend_from_slice(&chunk);
                }
                Ok(ThreadResListRaw { raw: Some(raw) })
            }
            _ => {
                counter!("dat_retrieval", "source" => "db").increment(1);
                let Some(board) = self.0.get_board(&input.board_key).await? else {
                    return Err(anyhow!("failed to find board"));
                };

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

                let th_res_list = ThreadResList {
                    thread: th,
                    res_list: responses,
                };

                Ok(ThreadResListRaw {
                    raw: Some(th_res_list.get_sjis_thread_res_list(&board.default_name)),
                })
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
