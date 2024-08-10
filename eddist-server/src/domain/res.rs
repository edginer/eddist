use std::{borrow::Cow, ops::Add};

use chrono::{DateTime, Datelike, Utc};
use eddist_core::domain::{client_info::ClientInfo, ip_addr::ReducedIpAddr};
use pwhash::unix;

use crate::{domain::metadent::Metadent, shiftjis::SJisStr};

use super::{
    authed_token::AuthedToken,
    metadent::MetadentType,
    res_core::ResCore,
    res_view::ResView,
    utils::{sanitize_base, sanitize_num_refs, to_ja_datetime},
};

pub trait ResState {}

pub enum AuthorIdInitialized {}
pub enum AuthorIdUninitialized {}

impl ResState for AuthorIdInitialized {}
impl ResState for AuthorIdUninitialized {}

#[derive(Debug, Clone)]
pub struct Res<T: ResState> {
    author_name: String,
    cap: Option<String>,
    trip: Option<String>,
    authed_token: Option<String>,
    author_id: Option<String>,
    created_at: DateTime<Utc>,
    mail: String,
    body: String,
    metadent_type: MetadentType,
    client_info: ClientInfo,
    is_abone: bool,
    is_email_authed: bool,
    board_key: String,
    _state: std::marker::PhantomData<T>,
    // NOTE: add is_mail_authed token when implement a feature distinguishes between mail_authed_token and cookie_authed_token
}

impl<T: ResState> Res<T> {
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

impl Res<AuthorIdUninitialized> {
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
            (sanitize_author_name(split.0), Some(split.1.to_string()))
        } else {
            (sanitize_author_name(from), None)
        };
        let (mail, cap, mail_authed_token) =
            if let Some((mail, after_delimiter)) = mail.split_once('#') {
                // #@ -> cap, # -> mail_authed_token
                if let Some(stripped) = after_delimiter.strip_prefix('@') {
                    (sanitize_email(mail), Some(stripped.to_string()), None)
                } else {
                    (
                        sanitize_email(mail),
                        None,
                        Some(after_delimiter.to_string()),
                    )
                }
            } else {
                (sanitize_email(mail), None, None)
            };

        let (is_email_authed, authed_token) = match (authed_token, mail_authed_token) {
            (Some(x), Some(_)) | (Some(x), _) => (false, Some(x)),
            (_, Some(x)) => (true, Some(x)),
            _ => (false, None), // No authed token same as is_email_authed = false
        };

        Self {
            author_name,
            cap,
            trip,
            mail,
            authed_token,
            author_id: None,
            created_at,
            body: sanitize_body(&body),
            metadent_type,
            client_info,
            is_abone,
            is_email_authed,
            board_key: board_key.to_string(),
            _state: std::marker::PhantomData,
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
                    Cow::Owned(body.replacen("!metadent:v:", "!metadent:v - configured", 1)),
                    MetadentType::Verbose,
                )
            } else if body.contains("!metadent:vv:") {
                (
                    Cow::Owned(body.replacen("!metadent:vv:", "!metadent:vv - configured", 1)),
                    MetadentType::VVerbose,
                )
            } else if body.contains("!metadent:vvv:") {
                (
                    Cow::Owned(body.replacen("!metadent:vvv:", "!metadent:vvv - configured", 1)),
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

    pub fn set_author_id(self, authed_token: &AuthedToken) -> Res<AuthorIdInitialized> {
        let author_id = get_author_id(
            &self.board_key,
            self.created_at,
            authed_token.reduced_ip.clone(),
        );

        Res {
            author_name: self.author_name,
            cap: self.cap,
            trip: self.trip,
            mail: self.mail,
            authed_token: self.authed_token,
            author_id: Some(author_id),
            created_at: self.created_at,
            body: self.body,
            metadent_type: self.metadent_type,
            client_info: self.client_info,
            is_abone: self.is_abone,
            is_email_authed: self.is_email_authed,
            board_key: self.board_key,
            _state: std::marker::PhantomData,
        }
    }
}

impl Res<AuthorIdInitialized> {
    pub fn author_id(&self) -> &str {
        self.author_id.as_ref().unwrap()
    }

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
                    self.pretty_author_name(default_name),
                    &mail,
                    &to_ja_datetime(self.created_at),
                    &self.author_id(),
                    &self.body,
                    thread_title.unwrap_or_default()
                )
                .as_str(),
            )
        }
    }
}

impl From<Res<AuthorIdInitialized>> for ResView {
    fn from(res: Res<AuthorIdInitialized>) -> Self {
        Self {
            author_name: res.author_name,
            mail: res.mail,
            body: res.body,
            created_at: res.created_at,
            author_id: res.author_id.unwrap(),
            is_abone: res.is_abone,
        }
    }
}

pub fn get_author_id(board_key: &str, datetime: DateTime<Utc>, ip_addr: ReducedIpAddr) -> String {
    let datetime = datetime.add(chrono::Duration::hours(9)); // JST
    let (year, month, day) = (datetime.year(), datetime.month(), datetime.day());
    calculate_trip(&format!("{year}-{month}-{day}:{board_key}:{ip_addr}"))
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

fn sanitize_author_name(author_name: &str) -> String {
    // Don't need to replace numeric character references because author_name doesn't contain it
    sanitize_base(author_name, false)
        .replace("★", "☆")
        .replace("◆", "◇")
}

fn sanitize_email(email: &str) -> String {
    // Don't need to replace numeric character references because email doesn't contain it
    sanitize_base(email, false)
}

fn sanitize_body(body: &str) -> String {
    sanitize_num_refs(&sanitize_base(body, true))
}
