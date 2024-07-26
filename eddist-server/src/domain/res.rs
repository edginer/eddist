use std::ops::Add;

use chrono::{DateTime, Datelike, Utc};
use pwhash::unix;

use crate::{domain::metadent::Metadent, shiftjis::SJisStr};

use super::{
    client_info::ClientInfo,
    ip_addr::{IpAddr, ReducedIpAddr},
    metadent::MetadentType,
    utils::to_ja_datetime,
};

#[derive(Debug, Clone)]
pub struct ResCore<'a> {
    pub from: &'a str,
    pub mail: &'a str,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct Res {
    author_name: String,
    cap: Option<String>,
    trip: Option<String>,
    authed_token: Option<String>,
    author_id: String,
    created_at: DateTime<Utc>,
    mail: String,
    body: String,
    metadent_type: MetadentType,
    client_info: ClientInfo,
    is_abone: bool,
    // NOTE: add is_mail_authed token when implement a feature distinguishes between mail_authed_token and cookie_authed_token
}

impl Res {
    pub fn new_from_res(
        ResCore { from, mail, body }: ResCore,
        board_key: &str,
        created_at: DateTime<Utc>,
        metadent_type: MetadentType,
        client_info: ClientInfo,
        authed_token: Option<String>,
        is_abone: bool,
    ) -> Self {
        let (author_name, trip) = if let Some(split) = from.split_once('#') {
            (split.0.to_string(), Some(split.1.to_string()))
        } else {
            (from.to_string(), None)
        };
        let (mail, cap, mail_authed_token) =
            if let Some((mail, after_delimiter)) = mail.split_once('#') {
                if let Some(stripped) = after_delimiter.strip_prefix('@') {
                    (mail.to_string(), Some(stripped.to_string()), None)
                } else {
                    (mail.to_string(), None, Some(after_delimiter.to_string()))
                }
            } else {
                (mail.to_string(), None, None)
            };
        let author_id = get_author_id(board_key, created_at, client_info.ip_addr());

        let authed_token = match (authed_token, mail_authed_token) {
            (Some(x), Some(_)) | (Some(x), _) => Some(x),
            (_, Some(x)) => Some(x),
            _ => None,
        };

        Self {
            author_name,
            cap,
            trip,
            mail,
            authed_token,
            author_id,
            created_at,
            body,
            metadent_type,
            client_info,
            is_abone,
        }
    }

    pub fn new_from_thread(
        ResCore { from, mail, body }: ResCore,
        board_key: &str,
        created_at: DateTime<Utc>,
        client_info: ClientInfo,
        authed_token: Option<String>,
        is_abone: bool,
    ) -> Self {
        let (body, metadent_type) = if body.contains("!metadent:") {
            if body.contains("!metadent:v:") {
                (
                    body.replacen("!metadent:v:", "!metadent:v - configured", 1),
                    MetadentType::Verbose,
                )
            } else if body.contains("!metadent:vv:") {
                (
                    body.replacen("!metadent:vv:", "!metadent:vv - configured", 1),
                    MetadentType::VVerbose,
                )
            } else if body.contains("!metadent:vvv:") {
                (
                    body.replacen("!metadent:vvv:", "!metadent:vvv - configured", 1),
                    MetadentType::VVerbose,
                )
            } else {
                (body, MetadentType::None)
            }
        } else {
            (body, MetadentType::None)
        };
        Self::new_from_res(
            ResCore { from, mail, body },
            board_key,
            created_at,
            metadent_type,
            client_info,
            authed_token,
            is_abone,
        )
    }

    pub fn get_sjis_bytes(&self, default_name: &str, thread_title: Option<&str>) -> SJisStr {
        let mail = if self.mail == "sage" { "sage" } else { "" };

        self.pretty_author_name(default_name).to_string();

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
                    self.pretty_author_name(default_name),
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

    pub fn pretty_author_name(&self, default_name: &str) -> String {
        let author_name = if self.author_name.is_empty() {
            default_name
        } else {
            &self.author_name
        };

        // base_name [ ★ cap_name ] [ ◆ trip_name ] [</b>(metadent info)<b>]
        let metadent = Metadent::new(self.metadent_type, &self.client_info, self.created_at);

        format!(
            "{}{}{}{}{}",
            author_name,
            self.cap
                .as_ref()
                .map(|x| format!(" ★{x}"))
                .unwrap_or_default(),
            self.trip
                .as_ref()
                .map(|x| format!(" ◆{x}"))
                .unwrap_or_default(),
            if self.metadent_type == MetadentType::None {
                ""
            } else {
                " "
            },
            metadent,
        )
    }

    pub fn author_id(&self) -> &str {
        &self.author_id
    }

    pub fn mail(&self) -> &str {
        &self.mail
    }

    pub fn body(&self) -> &str {
        &self.body
    }

    pub fn author_name(&self) -> &str {
        &self.author_name
    }

    pub fn authed_token(&self) -> Option<&String> {
        self.authed_token.as_ref()
    }
}

pub fn get_author_id(board_key: &str, datetime: DateTime<Utc>, ip_addr: IpAddr) -> String {
    let datetime = datetime.add(chrono::Duration::hours(9)); // JST
    let (year, month, day) = (datetime.year(), datetime.month(), datetime.day());
    let reduced = ReducedIpAddr::from(ip_addr);
    calculate_trip(&format!("{year}-{month}-{day}:{board_key}:{reduced}"))
}

// &str is utf-8 bytes
pub fn calculate_trip(target: &str) -> String {
    let bytes = encoding_rs::SHIFT_JIS.encode(target).0.into_owned();

    let mut salt = Vec::from(if bytes.len() >= 3 { &bytes[1..=2] } else { &[] });
    salt.push(0x48);
    salt.push(0x2e);
    let salt = salt
        .into_iter()
        .map(|x| match x {
            0x3a..=0x40 => x + 7,
            0x5b..=0x60 => x + 6,
            46..=122 => x,
            _ => 0x2e,
        })
        .collect::<Vec<_>>();

    let salt = std::str::from_utf8(&salt).unwrap();
    let result = unix::crypt(bytes.as_slice(), salt).unwrap();
    result[3..].to_string()
}
