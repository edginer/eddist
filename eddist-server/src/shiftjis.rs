use std::collections::HashMap;

use axum::{
    http::{HeaderName, HeaderValue},
    response::{IntoResponse, Response},
};
use axum_extra::extract::{CookieJar, cookie::Cookie};
use eddist_core::domain::sjis_str::SJisStr;
use hyper::StatusCode;

pub fn shift_jis_url_encodeded_body_to_vec(data: &str) -> Result<HashMap<&str, String>, ()> {
    fn ascii_hex_digit_to_byte(value: u8) -> Result<u8, ()> {
        if value.is_ascii_hexdigit() {
            if value.is_ascii_digit() {
                // U+0030 '0' - U+0039 '9',
                Ok(value - 0x30)
            } else if value.is_ascii_uppercase() {
                // U+0041 'A' - U+0046 'F',
                Ok(value - 0x41 + 0xa)
            } else if value.is_ascii_lowercase() {
                // U+0061 'a' - U+0066 'f',
                Ok(value - 0x61 + 0xa)
            } else {
                Err(())
            }
        } else {
            Err(())
        }
    }

    data.split('&')
        .map(|x| {
            let split = x.split('=').collect::<Vec<_>>();
            if split.len() != 2 {
                return std::result::Result::Err(());
            }
            let (key, value) = (split[0], split[1]);
            let bytes = value.as_bytes();
            let len = bytes.len();
            let mut i = 0;
            let mut result = Vec::new();
            while i < len {
                let item = bytes[i];
                if item == 0x25 {
                    // Look up the next two bytes from 0x25
                    if let Some([next1, next2]) = bytes.get(i + 1..i + 3) {
                        let first_byte = ascii_hex_digit_to_byte(*next1)?;
                        let second_byte = ascii_hex_digit_to_byte(*next2)?;
                        let code = first_byte * 0x10_u8 + second_byte;
                        result.push(code);
                    }
                    i += 2;
                } else if item == 0x2b {
                    result.push(0x20);
                } else {
                    result.push(bytes[i]);
                }
                i += 1;
            }
            let result = encoding_rs::SHIFT_JIS.decode(&result).0.to_string();
            Ok((key, result))
        })
        .collect::<Result<HashMap<_, _>, ()>>()
}

pub struct SJisResponse(Response);

impl From<SJisStr> for SJisResponse {
    fn from(value: SJisStr) -> Self {
        SJisResponse(Response::new(value.get_inner().into()))
    }
}

impl IntoResponse for SJisResponse {
    fn into_response(self) -> Response {
        self.0
    }
}

pub struct SJisResponseBuilder {
    body: SJisStr,
    s_max_age: usize,
    max_age: Option<usize>,
    content_type: SjisContentType,
    status_code: StatusCode,
    cookies: CookieJar,
    headers: Vec<(String, String)>,
}

pub enum SjisContentType {
    TextPlain,
    TextHtml,
}

impl SJisResponseBuilder {
    pub fn new(sjis_str: SJisStr) -> Self {
        SJisResponseBuilder {
            body: sjis_str,
            s_max_age: 0,
            max_age: None,
            content_type: SjisContentType::TextPlain,
            status_code: StatusCode::OK,
            cookies: CookieJar::new(),
            headers: Vec::new(),
        }
    }

    pub fn server_ttl(self, max_age: usize) -> Self {
        Self {
            s_max_age: max_age,
            ..self
        }
    }

    pub fn client_ttl(self, max_age: usize) -> Self {
        Self {
            max_age: Some(max_age),
            ..self
        }
    }

    pub fn content_type(self, content_type: SjisContentType) -> Self {
        Self {
            content_type,
            ..self
        }
    }

    pub fn status_code(self, status_code: StatusCode) -> Self {
        Self {
            status_code,
            ..self
        }
    }

    pub fn add_set_cookie(self, key: String, value: String, max_age: time::Duration) -> Self {
        let mut cookie = Cookie::new(key, value);
        cookie.set_http_only(true);
        cookie.set_max_age(max_age);
        cookie.set_path("/");
        let cookies = self.cookies.add(cookie);
        Self { cookies, ..self }
    }

    pub fn add_header(self, key: String, value: String) -> Self {
        let mut headers = self.headers;
        headers.push((key, value));
        Self { headers, ..self }
    }

    pub fn build(self) -> SJisResponse {
        let mut resp = Response::new(self.body.get_inner().into());
        let headers = resp.headers_mut();
        headers.clear();

        let cache_control_value = if let Some(max_age) = self.max_age {
            format!("max-age={},s-maxage={}", max_age, self.s_max_age)
        } else {
            format!("s-maxage={}", self.s_max_age)
        };
        headers.append(
            "Cache-Control",
            HeaderValue::from_str(&cache_control_value).unwrap(),
        );

        headers.append(
            "Content-Type",
            HeaderValue::from_str(match self.content_type {
                SjisContentType::TextPlain => "text/plain; charset=Shift_JIS;",
                SjisContentType::TextHtml => "text/html; charset=Shift_JIS;",
            })
            .unwrap(),
        );
        for cookie in self.cookies.iter() {
            headers.append(
                "Set-Cookie",
                HeaderValue::from_str(&cookie.to_string()).unwrap(),
            );
        }

        for (key, value) in self.headers {
            if let Ok(header_name) = HeaderName::try_from(key) {
                if let Ok(header_value) = HeaderValue::from_str(&value) {
                    headers.append(header_name, header_value);
                }
            }
        }

        let status_code = resp.status_mut();
        *status_code = self.status_code;

        SJisResponse(resp)
    }
}
