use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::utils::to_ja_datetime;

use super::{client_info::ClientInfo, sjis_str::SJisStr};

#[derive(Debug, Clone)]
pub struct ResView {
    pub author_name: String,
    pub mail: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub author_id: String,
    pub is_abone: bool,
}

#[derive(Debug, Clone)]
pub struct ResViewRef<'a> {
    pub author_name: &'a str,
    pub mail: &'a str,
    pub body: &'a str,
    pub created_at: DateTime<Utc>,
    pub author_id: &'a str,
    pub is_abone: bool,
}

pub fn get_sjis_bytes(
    res_view: ResViewRef<'_>,
    default_name: &str,
    thread_title: Option<&str>,
) -> SJisStr {
    let mail = if res_view.mail == "sage" { "sage" } else { "" };
    if res_view.is_abone {
        SJisStr::from(
            format!(
                "あぼーん<>あぼーん<><> あぼーん<> {}\n",
                thread_title.unwrap_or_default()
            )
            .as_str(),
        )
    } else {
        SJisStr::from(
            format!(
                "{}<>{}<>{} ID:{}<> {}<> {}\n",
                if res_view.author_name.is_empty() {
                    default_name
                } else {
                    &res_view.author_name
                },
                &mail,
                &to_ja_datetime(res_view.created_at),
                &res_view.author_id,
                &res_view.body,
                thread_title.unwrap_or_default()
            )
            .as_str(),
        )
    }
}

impl ResView {
    pub fn get_sjis_bytes(&self, default_name: &str, thread_title: Option<&str>) -> SJisStr {
        get_sjis_bytes(
            ResViewRef {
                author_name: &self.author_name,
                mail: &self.mail,
                body: &self.body,
                created_at: self.created_at,
                author_id: &self.author_id,
                is_abone: self.is_abone,
            },
            default_name,
            thread_title,
        )
    }

    pub fn get_sjis_admin_bytes(
        &self,
        default_name: &str,
        thread_title: Option<&str>,
        client_info: &ClientInfo,
        authed_token_id: Uuid,
    ) -> SJisStr {
        SJisStr::from(
            format!(
                "{}<>{}<>{} ID:{}<>{}<>{}<> {}<> {}\n",
                if self.author_name.is_empty() {
                    default_name
                } else {
                    &self.author_name
                },
                &self.mail,
                &to_ja_datetime(self.created_at),
                &self.author_id,
                &client_info.ip_addr,
                &authed_token_id,
                &self.body,
                thread_title.unwrap_or_default()
            )
            .as_str(),
        )
    }
}
