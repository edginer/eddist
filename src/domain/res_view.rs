use chrono::{DateTime, Utc};

use crate::shiftjis::SJisStr;

use super::utils::to_ja_datetime;

#[derive(Debug, Clone)]
pub struct ResView {
    pub author_name: String,
    pub mail: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub author_id: String,
    pub is_abone: bool,
}

impl ResView {
    pub fn get_sjis_bytes(&self, default_name: &str, thread_title: Option<&str>) -> SJisStr {
        let mail = if self.mail == "sage" { "sage" } else { "" };
        if self.is_abone {
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
                    if self.author_name.is_empty() {
                        default_name
                    } else {
                        &self.author_name
                    },
                    &mail,
                    &to_ja_datetime(self.created_at),
                    &self.author_id,
                    &self.body,
                    thread_title.unwrap_or_default()
                )
                .as_str(),
            )
        }
    }
}
