use core::panic;
use std::{borrow::Cow, ops::Add};

use base64::Engine;
use chrono::{DateTime, Datelike, Utc};
use eddist_core::domain::{
    client_info::ClientInfo,
    ip_addr::ReducedIpAddr,
    res::{ResView, ResViewRef},
    sjis_str::SJisStr,
};
use md5::Md5;
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
    urls: Option<Vec<String>>,
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

    pub fn is_email_authed(&self) -> bool {
        self.is_email_authed
    }

    pub fn metadent_type(&self) -> MetadentType {
        self.metadent_type
    }

    pub fn get_all_urls(&mut self) -> Vec<String> {
        let text = &self.body;
        let mut urls = Vec::new();

        let prefixes = [
            "https://", "http://", "ttp://", "ttps://", "tps://", "tp://", "p://", "ps://", "s://",
            "://", "//",
        ];

        let no_slsl_prefixes = [
            "https:", "http:", "ttp:", "ttps:", "tps:", "tp:", "p:", "ps:", "s:", ":", "",
        ];

        let lines = text.split("<br>");
        for line in lines {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Extract URLs with explicit scheme prefixes
            let mut pos = 0;

            while let Some(slsl_idx) = line.get(pos..).and_then(|x| x.find("//")) {
                // Check previous characters of "//" to find the prefix
                let slsl_idx = slsl_idx + pos;
                let mut found_prefix = None;
                for prefix in no_slsl_prefixes.iter() {
                    if line.get(pos..slsl_idx).unwrap().ends_with(prefix) {
                        found_prefix = Some(prefix);
                        break;
                    }
                }

                let found_index = match found_prefix {
                    Some(prefix) => slsl_idx - prefix.len(),
                    None => slsl_idx,
                };

                let substring = line.get(found_index..).unwrap();
                let end_offset = substring
                    .find(|c: char| c.is_ascii_control() || c.is_whitespace() || !c.is_ascii())
                    .unwrap_or(substring.len());
                let url = substring.get(..end_offset).unwrap();
                urls.push(url.to_string());
                pos = found_index + end_offset;
            }

            // Additionally, detect plain domain URLs like "example.com"
            for token in line
                .split(|c: char| c.is_ascii_control() || c.is_whitespace() || !c.is_ascii())
                .filter(|t| t.contains('.'))
            {
                if !token.contains("//") && token.contains('.') && token.is_ascii() {
                    urls.push(token.to_string());
                }
            }
        }

        self.urls = Some(
            urls.into_iter()
                .map(|url| {
                    let url_core = prefixes
                        .iter()
                        .fold(url, |acc, prefix| acc.replace(prefix, ""));

                    let (domain, has_path) = if let Some((domain, path)) = url_core.split_once('/')
                    {
                        (domain, !path.is_empty())
                    } else {
                        (&url_core as &str, false)
                    };

                    if has_path {
                        url_core
                    } else {
                        domain
                            .chars()
                            .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '.')
                            .collect()
                    }
                })
                .collect(),
        );

        self.urls.clone().unwrap_or_default()
    }

    pub fn get_all_images(&mut self) -> Vec<String> {
        if self.urls.is_none() {
            self.get_all_urls();
        }

        let mut images = Vec::new();
        if let Some(urls) = &self.urls {
            for url in urls {
                if url.ends_with(".jpg")
                    || url.ends_with(".jpeg")
                    || url.ends_with(".png")
                    || url.ends_with(".gif")
                    || url.ends_with(".webp")
                    || url.contains("imgur.com")
                {
                    images.push(url.clone());
                }
            }
        }
        images
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
            urls: None,
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
                    MetadentType::VVVerbose,
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
        let author_id = if retrieved_cap_name.is_none() {
            get_author_id_with_device_info(
                &self.board_key,
                self.created_at,
                &authed_token.author_id_seed,
                Some(&authed_token.writing_ua),
                &authed_token.reduced_ip,
            )
        } else {
            "????".to_string()
        };

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
            urls: self.urls,
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
        let author_name = if self.author_name.is_empty() && self.cap.is_none() {
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

// Character set for ID generation (base64-like encoding)
const ID_CHAR_SET: &[char] = &[
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S',
    'T', 'U', 'V', 'W', 'X', 'Y', 'Z', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l',
    'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '0', '1', '2', '3', '4',
    '5', '6', '7', '8', '9', '.', '/',
];

/// Extract the UA part before the first parenthesis
fn extract_ua_prefix(ua: &str) -> &str {
    ua.split('(').next().unwrap_or(ua).trim()
}

/// Generate device-specific suffix characters
/// Returns (ua_char_opt, ip_char_opt) based on available data
fn generate_device_suffix(
    ua: Option<&str>,
    reduced_ip: Option<&ReducedIpAddr>,
) -> (Option<char>, Option<char>) {
    let ua_char = ua.map(|ua| {
        let ua_prefix = extract_ua_prefix(ua);
        let ua_hash = Md5::digest(ua_prefix.as_bytes());
        let ua_idx = ua_hash[0] as usize % ID_CHAR_SET.len();
        ID_CHAR_SET[ua_idx]
    });

    let ip_char = reduced_ip.map(|ip| {
        let ip_str = ip.to_string();
        let ip_first_segment = ip_str
            .split('.')
            .next()
            .or_else(|| ip_str.split(':').next())
            .unwrap_or("");
        let ip_hash = Md5::digest(ip_first_segment.as_bytes());
        let ip_idx = ip_hash[0] as usize % ID_CHAR_SET.len();
        ID_CHAR_SET[ip_idx]
    });

    (ua_char, ip_char)
}

/// Generate ID with seed and optional device-specific suffix
pub fn generate_id_with_device_suffix(
    seed_id: &str,
    length: usize,
    ua: Option<&str>,
    reduced_ip: Option<&ReducedIpAddr>,
) -> String {
    if length < 2 {
        panic!("Length must be at least 2");
    }
    let mut id_chars: Vec<char> = seed_id.chars().collect();

    // Ensure we have enough characters
    if id_chars.len() < length {
        return seed_id[..id_chars.len().min(length)].to_string();
    }

    let (ua_char, ip_char) = generate_device_suffix(ua, reduced_ip);

    match (ua_char, ip_char) {
        (Some(ua), Some(ip)) => {
            id_chars[length - 2] = ua;
            id_chars[length - 1] = ip;
        }
        (None, Some(ip)) => {
            id_chars[length - 1] = ip;
        }
        (Some(ua), None) => {
            id_chars[length - 1] = ua;
        }
        _ => {}
    }

    id_chars[..length].iter().collect()
}

pub fn get_author_id_by_seed(board_key: &str, datetime: DateTime<Utc>, seed: &[u8]) -> String {
    let datetime = datetime.add(chrono::Duration::hours(9)); // JST
    let (year, month, day) = (datetime.year(), datetime.month(), datetime.day());
    calculate_trip(&format!("{year}-{month}-{day}:{board_key}:{seed:?}"))
}

pub fn get_author_id_with_device_info(
    board_key: &str,
    datetime: DateTime<Utc>,
    seed: &[u8],
    ua: Option<&str>,
    reduced_ip: &ReducedIpAddr,
) -> String {
    let base_id = get_author_id_by_seed(board_key, datetime, seed);
    generate_id_with_device_suffix(&base_id, 9, ua, Some(reduced_ip))
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
        .replace("&starf;", "☆")
        .replace("&bigstar;", "☆")
}

fn sanitize_email(email: &str) -> String {
    // Don't need to replace numeric character references because email doesn't contain it
    sanitize_base(email, false)
}

fn sanitize_body(body: &str) -> String {
    sanitize_num_refs(&sanitize_base(body, true))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    struct Dummy;
    impl ResState for Dummy {}

    #[test]
    fn test_get_all_urls() {
        let content = r#"Visit http://example.com 
and also check https://rust-lang.org, then
plainexample.com appears and finally ttp://fake.com/aaa.vvv for a test
あいうえおexample2.com,あいうえお
あいうえおexample3.comあいうえお
あいうえおps://example4.comあいうえお
//example5.comあいうえおexample6.com
         "#
        .replace("\n", "<br>");
        let mut res = Res::<Dummy> {
            author_name: "".to_string(),
            cap: None,
            trip: None,
            authed_token: None,
            author_id: None,
            created_at: Utc::now(),
            mail: "".to_string(),
            body: content.to_string(),
            metadent_type: MetadentType::None,
            client_info: ClientInfo {
                user_agent: "".to_string(),
                asn_num: 0,
                ip_addr: "".to_string(),
                tinker: None,
            },
            is_abone: false,
            is_email_authed: false,
            board_key: "".to_string(),
            urls: None,
            _state: std::marker::PhantomData,
        };
        let urls = res.get_all_urls();
        let mut urls_sorted = urls.clone();
        urls_sorted.sort();
        let mut expected = vec![
            "example.com".to_string(),
            "rust-lang.org".to_string(),
            "plainexample.com".to_string(),
            "fake.com/aaa.vvv".to_string(),
            "example2.com".to_string(),
            "example3.com".to_string(),
            "example4.com".to_string(),
            "example5.com".to_string(),
            "example6.com".to_string(),
        ];
        expected.sort();
        assert_eq!(expected, urls_sorted);
    }

    #[test]
    fn test_get_all_images() {
        let content = "Image: https://example.com/pic.png page: https://example.com/page.html another non-image: http://site.org/image.jpg, non-image: plainexample.com<br>imgur.com/abc123<br>https://example.com/another.png";
        let mut res = Res::<Dummy> {
            author_name: "".to_string(),
            cap: None,
            trip: None,
            authed_token: None,
            author_id: None,
            created_at: Utc::now(),
            mail: "".to_string(),
            body: content.to_string(),
            metadent_type: MetadentType::None,
            client_info: ClientInfo {
                user_agent: "".to_string(),
                asn_num: 0,
                ip_addr: "".to_string(),
                tinker: None,
            },
            is_abone: false,
            is_email_authed: false,
            board_key: "".to_string(),
            urls: None,
            _state: std::marker::PhantomData,
        };
        let images = res.get_all_images();
        let mut expected = vec![
            "example.com/pic.png".to_string(),
            "imgur.com/abc123".to_string(),
            "example.com/another.png".to_string(),
        ];
        expected.sort();
        let mut images_sorted = images.clone();
        images_sorted.sort();
        assert_eq!(expected, images_sorted);
    }
}
