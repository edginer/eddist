use eddist_core::domain::client_info::ClientInfo;
use uuid::Uuid;

use crate::error::{BbsCgiError, ContentLengthExceededParamType};

use super::res_core::ResCore;

#[derive(Debug, Clone)]
pub struct Board {
    pub id: Uuid,
    pub name: String,
    pub board_key: String,
    pub default_name: String,
}

#[derive(Debug, Clone)]
pub struct BoardInfo {
    pub id: Uuid,
    pub local_rules: String,
    pub base_thread_creation_span_sec: i32,
    pub base_response_creation_span_sec: i32,
    pub max_thread_name_byte_length: i32,
    pub max_author_name_byte_length: i32,
    pub max_email_byte_length: i32,
    pub max_response_body_byte_length: i32,
    pub max_response_body_lines: i32,
    pub threads_archive_cron: Option<String>,
    pub threads_archive_trigger_thread_count: Option<i32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
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
        let l_count = self.body.lines().count();
        if l_count > board_info.max_response_body_lines as usize {
            return Err(BbsCgiError::ContentLengthExceeded(
                ContentLengthExceededParamType::BodyLines,
            ));
        }

        Ok(())
    }
}

impl BoardInfoResRestrictable for (&ResCore<'_>, &str) {
    fn validate_content_length(&self, board_info: &BoardInfo) -> Result<(), BbsCgiError> {
        let (res, thread_name) = self;

        res.validate_content_length(board_info)?;

        if thread_name.len() > board_info.max_thread_name_byte_length as usize {
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
            if chrono::Utc::now().timestamp() as u64 - tinker.last_wrote_at()
                < board_info.base_response_creation_span_sec as u64
            {
                return Err(BbsCgiError::TooManyCreatingRes(
                    board_info.base_response_creation_span_sec,
                ));
            }
            if is_thread {
                if let Some(last_created_thread_at) = tinker.last_created_thread_at() {
                    let span = chrono::Utc::now().timestamp() as u64 - last_created_thread_at;
                    let level_map = [60 * 60, 60 * 30, 60 * 15, 60 * 8, 60 * 4, 60 * 2];
                    let span_limit = level_map[if tinker.level() > 5 {
                        5
                    } else {
                        tinker.level()
                    } as usize]
                        * 2;

                    if span < span_limit {
                        return Err(BbsCgiError::TooManyCreatingThread {
                            tinker_level: tinker.level(),
                            span_sec: board_info.base_thread_creation_span_sec,
                        });
                    }
                }
            }

            Ok(())
        } else {
            if is_thread {
                return Err(BbsCgiError::TmpCanNotCreateThread);
            }
            Ok(())
        }
    }
}
