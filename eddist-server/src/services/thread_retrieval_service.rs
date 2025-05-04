use anyhow::anyhow;
use redis::{aio::ConnectionManager, Cmd, Value};

use crate::{
    domain::thread_res_list::ThreadResList, repositories::bbs_repository::BbsRepository,
    utils::redis::thread_cache_key,
};

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
            .send_packed_command(&Cmd::lrange(
                thread_cache_key(&input.board_key, input.thread_number),
                0,
                -1,
            ))
            .await
            .unwrap()
            .into_sequence()
        {
            Ok(sjis_result) if !sjis_result.is_empty() => {
                let sjis_result = sjis_result
                    .into_iter()
                    .filter_map(|x| {
                        if let Value::Data(data) = x {
                            Some(data)
                        } else {
                            None
                        }
                    })
                    .flatten()
                    .collect::<Vec<_>>();

                Ok(ThreadResListRaw { raw: sjis_result })
            }
            _ => {
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
                    raw: th_res_list.get_sjis_thread_res_list(&board.default_name),
                })
            }
        }
    }
}

pub struct ThreadRetrievalServiceInput {
    pub board_key: String,
    pub thread_number: u64,
}

#[derive(Debug, Clone)]
pub struct ThreadResListRaw {
    raw: Vec<u8>,
}

impl ThreadResListRaw {
    pub fn raw(self) -> Vec<u8> {
        self.raw
    }
}
