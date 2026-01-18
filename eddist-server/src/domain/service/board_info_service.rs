use std::{
    collections::HashMap,
    env,
    sync::{OnceLock, RwLock},
};

use eddist_core::{
    cache_aside::AsCacheRef,
    domain::{
        board::{Board, BoardInfo},
        client_info::ClientInfo,
    },
};
use uuid::Uuid;

use crate::{
    domain::res_core::ResCore,
    error::{BbsCgiError, ContentEmptyParamType, ContentLengthExceededParamType},
    repositories::bbs_repository::BbsRepository,
};

/// Maximum characters allowed per line in body content
const MAX_CHARS_PER_LINE: usize = 250;

struct BoardInfoLocalCacheItem {
    board_info: BoardInfo,
    expired_at: u64,
}

impl AsCacheRef<BoardInfo> for BoardInfoLocalCacheItem {
    fn expired_at(&self) -> u64 {
        self.expired_at
    }

    fn get(&self) -> &BoardInfo {
        &self.board_info
    }
}

pub struct BoardInfoService<T: BbsRepository>(T);

impl<T: BbsRepository + Clone> BoardInfoService<T> {
    pub fn new(repo: T) -> Self {
        Self(repo)
    }

    pub async fn get_board_info_by_id(&self, board_id: Uuid) -> anyhow::Result<Option<BoardInfo>> {
        static BOARD_INFO_LOCAL_CACHE: OnceLock<RwLock<HashMap<Uuid, BoardInfoLocalCacheItem>>> =
            OnceLock::new();
        let cache = BOARD_INFO_LOCAL_CACHE.get_or_init(|| RwLock::new(HashMap::new()));

        match cache.read() {
            Ok(cache) => {
                if let Some(cache) = cache.get(&board_id) {
                    if cache.expired_at() > chrono::Utc::now().timestamp() as u64 {
                        return Ok(Some(cache.get().clone()));
                    }
                }
            }
            Err(e) => return Err(anyhow::anyhow!("failed to read cache: {e:?}")),
        }

        let board_info = self
            .0
            .get_board_info(board_id)
            .await
            .map_err(|_| anyhow::anyhow!("failed to find board info"))?;
        if board_info.is_none() {
            return Ok(None);
        }

        let mut write_lock = cache
            .write()
            .map_err(|e| anyhow::anyhow!("failed to write cache: {e:?}"))?;

        write_lock.insert(
            board_id,
            BoardInfoLocalCacheItem {
                board_info: board_info.clone().unwrap(),
                expired_at: chrono::Utc::now().timestamp() as u64 + 60,
            },
        );

        Ok(board_info)
    }

    pub async fn get_board_info_by_key(
        &self,
        board_key: &str,
    ) -> anyhow::Result<Option<(Board, BoardInfo)>> {
        let board = self
            .0
            .get_board(board_key)
            .await
            .map_err(|_| anyhow::anyhow!("failed to find board info"))?
            .ok_or_else(|| anyhow::anyhow!("failed to find board info"))?;

        let board_info = self
            .get_board_info_by_id(board.id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("failed to find board info"))?;

        Ok(Some((board, board_info)))
    }
}

pub trait BoardInfoResRestrictable {
    fn validate_content_length(&self, board_info: &BoardInfo) -> Result<(), BbsCgiError>;
}

impl BoardInfoResRestrictable for ResCore<'_> {
    fn validate_content_length(&self, board_info: &BoardInfo) -> Result<(), BbsCgiError> {
        if self.body.len() > board_info.max_response_body_byte_length as usize {
            return Err(BbsCgiError::ContentLengthExceeded(
                ContentLengthExceededParamType::Body,
            ));
        }
        if self.from.len() > board_info.max_author_name_byte_length as usize {
            return Err(BbsCgiError::ContentLengthExceeded(
                ContentLengthExceededParamType::Name,
            ));
        }
        if self.mail.len() > board_info.max_email_byte_length as usize {
            return Err(BbsCgiError::ContentLengthExceeded(
                ContentLengthExceededParamType::Mail,
            ));
        }

        let lines = self.body.lines().collect::<Vec<_>>();
        let l_count = lines.len();
        if l_count > board_info.max_response_body_lines as usize {
            return Err(BbsCgiError::ContentLengthExceeded(
                ContentLengthExceededParamType::BodyLines,
            ));
        }

        if lines
            .iter()
            .any(|line| line.chars().count() > MAX_CHARS_PER_LINE)
        {
            return Err(BbsCgiError::ContentLengthExceeded(
                ContentLengthExceededParamType::BodyLineLength,
            ));
        }

        Ok(())
    }
}

impl BoardInfoResRestrictable for (&ResCore<'_>, &str) {
    fn validate_content_length(&self, board_info: &BoardInfo) -> Result<(), BbsCgiError> {
        let (res, thread_name) = self;

        res.validate_content_length(board_info)?;

        if thread_name.is_empty() {
            return Err(BbsCgiError::ContentEmpty(ContentEmptyParamType::ThreadName));
        }

        if thread_name.len() > board_info.max_thread_name_byte_length as usize {
            return Err(BbsCgiError::ContentLengthExceeded(
                ContentLengthExceededParamType::ThreadName,
            ));
        }

        // Restrict length of character (not byte) only for thread name, using max_thread_name_byte_length / 2
        if thread_name.chars().count() > board_info.max_thread_name_byte_length as usize / 2 {
            return Err(BbsCgiError::ContentLengthExceeded(
                ContentLengthExceededParamType::ThreadName,
            ));
        }

        Ok(())
    }
}

pub trait BoardInfoClientInfoResRestrictable {
    fn validate_client_info(
        &self,
        board_info: &BoardInfo,
        is_thread: bool,
    ) -> Result<(), BbsCgiError>;
}

impl BoardInfoClientInfoResRestrictable for ClientInfo {
    fn validate_client_info(
        &self,
        board_info: &BoardInfo,
        is_thread: bool,
    ) -> Result<(), BbsCgiError> {
        if let Some(tinker) = &self.tinker {
            // Response creation span multiplier based on tinker level
            let span_multiplier = match tinker.level() {
                0 | 1 => 3,
                2 => 2,
                _ => 1, // level >= 3
            };
            let res_span_sec =
                (board_info.base_response_creation_span_sec * span_multiplier) as u64;

            let elapsed = chrono::Utc::now().timestamp() as u64 - tinker.last_wrote_at();
            if elapsed < res_span_sec {
                return Err(BbsCgiError::TooManyCreatingRes {
                    tinker_level: tinker.level(),
                    span_sec: res_span_sec as i32,
                });
            }

            if is_thread {
                if let Some(last_created_thread_at) = tinker.last_created_thread_at() {
                    let elapsed = chrono::Utc::now().timestamp() as u64 - last_created_thread_at;
                    // Thread creation span limit (in seconds) based on tinker level
                    // Base values are doubled for the final limit
                    let base_span = match tinker.level() {
                        0 => 60 * 60, // 1 hour
                        1 => 60 * 30, // 30 min
                        2 => 60 * 15, // 15 min
                        3 => 60 * 8,  // 8 min
                        4 => 60 * 4,  // 4 min
                        _ => 60 * 2,  // 2 min (level >= 5)
                    };
                    let span_limit = base_span * 2;

                    if elapsed < span_limit {
                        return Err(BbsCgiError::TooManyCreatingThread {
                            tinker_level: tinker.level(),
                            span_sec: span_limit as i32,
                        });
                    }
                }
            }

            Ok(())
        } else {
            if is_thread
                && env::var("RESTRICT_THREAD_CREATION_ON_NO_TINKER").unwrap_or("false".to_string())
                    == "true"
            {
                return Err(BbsCgiError::TmpCanNotCreateThread);
            }
            Ok(())
        }
    }
}
