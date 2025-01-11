use base64::Engine;
use chrono::TimeZone;
use md5::Digest;

use crate::{
    domain::{
        metadent::{generate_date_seed, generate_meta_ident},
        thread_list::ThreadListWithMetadent,
    },
    repositories::bbs_repository::BbsRepository,
};

use super::AppService;

#[derive(Debug, Clone)]
pub struct MetadentThreadListService<T: BbsRepository>(T);

impl<T: BbsRepository> MetadentThreadListService<T> {
    pub fn new(repo: T) -> Self {
        Self(repo)
    }
}

#[async_trait::async_trait]
impl<T: BbsRepository> AppService<BoardKey, ThreadListWithMetadent>
    for MetadentThreadListService<T>
{
    async fn execute(&self, input: BoardKey) -> anyhow::Result<ThreadListWithMetadent> {
        let board = self
            .0
            .get_board(&input.0)
            .await?
            .ok_or_else(|| anyhow::anyhow!("failed to find board info"))?;

        let threads = self.0.get_threads_with_metadent(board.id).await?;
        let threads = threads
            .into_iter()
            .map(|(thread, client_info)| {
                let thread_number = thread.thread_number;
                (thread, {
                    let metadent = generate_meta_ident(
                        client_info.asn_num,
                        &client_info
                            .tinker
                            .map(|x| if x.level() < 3 { 0 } else { x.level() })
                            .unwrap_or(0)
                            .to_string(),
                        &client_info.user_agent,
                        generate_date_seed(chrono::Utc.timestamp_opt(thread_number, 0).unwrap()),
                    );
                    let mut hasher = md5::Md5::new();
                    hasher.update(metadent.as_bytes());
                    // from u8; 16 to base64
                    let result = hasher.finalize();
                    let metadent =
                        base64::engine::general_purpose::STANDARD.encode(&result)[1..9].to_string();
                    metadent
                })
            })
            .collect();

        Ok(ThreadListWithMetadent {
            board,
            thread_list: threads,
        })
    }
}

#[derive(Debug, Clone)]
pub struct BoardKey(pub String);
