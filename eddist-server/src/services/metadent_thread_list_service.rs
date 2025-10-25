use base64::Engine;
use chrono::TimeZone;
use md5::Digest;
use std::ops::Add;

use crate::{
    domain::{
        metadent::{generate_date_seed, generate_meta_ident, METADENT_RESET_PERIOD_DAYS},
        res::{
            generate_id_with_device_suffix, get_author_id_by_seed,
            AUTHOR_ID_SUFFIX_RESET_PERIOD_DAYS,
        },
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
            .map(|(thread, client_info, authed_token)| {
                let thread_number = thread.thread_number;
                let thread_datetime = chrono::Utc.timestamp_opt(thread_number, 0).unwrap();

                // First 4 chars: based on writing info (current implementation)
                let writing_metadent = generate_meta_ident(
                    client_info.asn_num,
                    &client_info
                        .tinker
                        .map(|x| if x.level() < 3 { 0 } else { x.level() })
                        .unwrap_or(0)
                        .to_string(),
                    &client_info.user_agent,
                    generate_date_seed(thread_datetime, METADENT_RESET_PERIOD_DAYS),
                );
                let mut hasher = md5::Md5::new();
                hasher.update(writing_metadent.as_bytes());
                let result = hasher.finalize();
                let first_4 =
                    base64::engine::general_purpose::STANDARD.encode(result)[1..5].to_string();

                // Last 4 chars: based on authed token (first 2 from seed, last 2 device-specific)
                let author_id_base = get_author_id_by_seed(
                    &board.board_key,
                    thread_datetime,
                    &authed_token.author_id_seed,
                );
                let last_4 = generate_id_with_device_suffix(
                    &author_id_base,
                    4,
                    None,
                    Some(&authed_token.reduced_ip),
                    Some(generate_date_seed(
                        thread_datetime.add(chrono::Duration::hours(9)), // to JST,
                        AUTHOR_ID_SUFFIX_RESET_PERIOD_DAYS,
                    )),
                );

                (thread, format!("{first_4}{last_4}"))
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
