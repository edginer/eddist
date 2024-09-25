use std::{borrow::Cow, ops::Add};

use base64::Engine;
use chrono::{DateTime, Datelike, Utc};
use eddist_core::domain::{
    client_info::ClientInfo,
    ip_addr::ReducedIpAddr,
    res::{ResView, ResViewRef},
    sjis_str::SJisStr,
};
use pwhash::unix;
use sha1::{Digest, Sha1};

use crate::domain::metadent::Metadent;

use super::{
    authed_token::AuthedToken,
    metadent::MetadentType,
    res_core::ResCore,
    utils::{sanitize_base, sanitize_num_refs, SimpleSecret},
};

pub trait ResState {}

pub enum AuthorIdInitialized {}
pub enum AuthorIdUninitialized {}

impl ResState for AuthorIdInitialized {}
impl ResState for AuthorIdUninitialized {}

#[derive(Debug, Clone)]
enum CapState {
    RawPassword(SimpleSecret),
    Retrieved(String),
}

#[derive(Debug, Clone)]
pub struct Res<T: ResState> {
    author_name: String,
    cap: Option<CapState>,
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
}

impl<T: ResState> Res<T> {
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

    pub fn is_sage(&self) -> bool {
        self.mail == "sage"
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
            (sanitize_author_name(split.0), Some(calculate_trip(split.1)))
        } else {
            (sanitize_author_name(from), None)
        };
        let (mail, cap, mail_authed_token) =
            if let Some((mail, after_delimiter)) = mail.split_once('#') {
                // #@ -> cap, # -> mail_authed_token, cannot use both
                if let Some(stripped) = after_delimiter.strip_prefix('@') {
                    (
                        sanitize_email(mail),
                        Some(CapState::RawPassword(SimpleSecret::new(stripped))),
                        None,
                    )
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

    pub fn set_author_id(
        self,
        authed_token: &AuthedToken,
        retrieved_cap_name: Option<String>,
    ) -> Res<AuthorIdInitialized> {
        let author_id = get_author_id(
            &self.board_key,
            self.created_at,
            authed_token.reduced_ip.clone(),
        )[..9]
            .to_string();

        Res {
            author_name: self.author_name,
            cap: retrieved_cap_name.map(CapState::Retrieved),
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

    pub fn cap(&self) -> Option<&SimpleSecret> {
        match &self.cap {
            Some(CapState::RawPassword(secret)) => Some(secret),
            _ => None,
        }
    }
}

impl Res<AuthorIdInitialized> {
    pub fn author_id(&self) -> &str {
        self.author_id.as_ref().unwrap()
    }

    pub fn pretty_author_name(&self, default_name: &str) -> String {
        let author_name = if self.author_name.is_empty() {
            default_name
        } else {
            &self.author_name
        };

        // base_name [ ★ cap_name ] [ ◆ trip_name ] [</b>(metadent info)<b>]
        let metadent = Metadent::new(self.metadent_type, &self.client_info, self.created_at);
        let cap = if let Some(CapState::Retrieved(ref cap)) = self.cap {
            Some(cap)
        } else {
            None
        };

        format!(
            "{}{}{}{}{}",
            author_name,
            cap.as_ref().map(|x| format!(" ★{x}")).unwrap_or_default(),
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

    pub fn get_sjis_bytes(&self, default_name: &str, thread_title: Option<&str>) -> SJisStr {
        let res_view_ref = ResViewRef {
            author_name: &self.pretty_author_name(default_name),
            mail: self.mail(),
            body: self.body(),
            created_at: self.created_at,
            author_id: self.author_id(),
            is_abone: self.is_abone,
        };

        eddist_core::domain::res::get_sjis_bytes(res_view_ref, default_name, thread_title)
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

    if bytes.len() >= 12 {
        let mut hasher = Sha1::new();
        hasher.update(&bytes);

        let calc_bytes = Vec::from(hasher.finalize().as_slice());
        let result = &base64::engine::general_purpose::STANDARD.encode(calc_bytes)[0..12];
        result.to_string().replace('+', ".")
    } else {
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
