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
                "あぼーん<>あぼーん<><> あぼーん <>{}\n",
                thread_title.unwrap_or_default()
            )
            .as_str(),
        )
    } else {
        SJisStr::from(
            format!(
                "{}<>{}<>{} ID:{}<> {} <>{}\n",
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
                "{}<>{}<>{} ID:{}<>{}<>{}<> {} <>{}\n",
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
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_get_sjis_bytes_normal_post() {
        let res_view = ResViewRef {
            author_name: "テストユーザー",
            mail: "",
            body: "テスト投稿です",
            created_at: Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap(),
            author_id: "ABC123",
            is_abone: false,
        };

        let result = get_sjis_bytes(res_view, "名無しさん", Some("テストスレッド"));
        let output = result.to_string();

        assert!(output.contains("テストユーザー"));
        assert!(output.contains("テスト投稿です"));
        assert!(output.contains("ID:ABC123"));
        assert!(output.contains("テストスレッド"));
    }

    #[test]
    fn test_get_sjis_bytes_sage_post() {
        let res_view = ResViewRef {
            author_name: "sage投稿者",
            mail: "sage",
            body: "sage投稿",
            created_at: Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap(),
            author_id: "DEF456",
            is_abone: false,
        };

        let result = get_sjis_bytes(res_view, "名無しさん", None);
        let output = result.to_string();

        assert!(output.contains("sage投稿者<>sage<>"));
        assert!(output.contains("sage投稿"));
    }

    #[test]
    fn test_get_sjis_bytes_abone() {
        let res_view = ResViewRef {
            author_name: "荒らし",
            mail: "",
            body: "削除対象",
            created_at: Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap(),
            author_id: "XYZ789",
            is_abone: true,
        };

        let result = get_sjis_bytes(res_view, "名無しさん", Some("テストスレッド"));
        let output = result.to_string();

        assert!(output.contains("あぼーん"));
        assert!(!output.contains("荒らし"));
        assert!(!output.contains("削除対象"));
    }
}
