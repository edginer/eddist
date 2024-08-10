use std::fmt::Display;

use axum::response::{IntoResponse, Response};
use hyper::StatusCode;
use time::Duration;

use crate::{shiftjis::SJisStr, SJisResponseBuilder, SjisContentType};

#[derive(thiserror::Error, Debug)]
pub enum BbsCgiError {
    // this error occurs mainly when the client is developing the client
    #[error("bbs.cgiの呼び出しには'{0}'が必要です")]
    InsufficientClientRequest(InsufficientParamType),

    #[error("bbs.cgi呼び出し時の'{0}'が不正です")]
    InvalidClientRequestParameter(InvalidParamType),

    #[error("対象の'{0}'が見つかりません")]
    NotFound(NotFoundParamType),

    #[error("スレッドストッパーが働いたみたいなので書き込めません")]
    InactiveThread,

    #[error("既に同時刻にスレッドが作成されています")]
    SameTimeThreadCration,

    #[error("認証コード'{auth_code}'を用いて、以下のURLから認証を行ってください \n {base_url}/auth_code")]
    Unauthenticated {
        auth_code: String,
        base_url: String,
        auth_token: String,
    },

    // cause on failed to find authed token by given token (not found)
    #[error("与えられた認証トークンが不正です")]
    InvalidAuthedToken,

    #[error("その認証トークンは無効化されました")]
    RevokedAuthedToken,

    #[error("NGワードが含まれています")]
    NgWordDetected,

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl BbsCgiError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            BbsCgiError::InsufficientClientRequest(_) => StatusCode::BAD_REQUEST,
            BbsCgiError::InvalidClientRequestParameter(_) => StatusCode::BAD_REQUEST,
            BbsCgiError::NotFound(_) => StatusCode::OK,
            BbsCgiError::InactiveThread => StatusCode::OK,
            BbsCgiError::SameTimeThreadCration => StatusCode::OK,
            BbsCgiError::Unauthenticated { .. } => StatusCode::OK,
            BbsCgiError::InvalidAuthedToken => StatusCode::BAD_REQUEST,
            BbsCgiError::RevokedAuthedToken => StatusCode::FORBIDDEN,
            BbsCgiError::NgWordDetected => StatusCode::OK,
            BbsCgiError::Other(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for BbsCgiError {
    fn into_response(self) -> Response {
        let edge_token = if let BbsCgiError::Unauthenticated { auth_token, .. } = &self {
            Some(auth_token.to_string())
        } else {
            None
        };

        let status_code = self.status_code();
        let e = match self {
            BbsCgiError::Other(_) => "内部エラーが発生しました".to_string(),
            e => e.to_string(),
        };

        let resp = SJisResponseBuilder::new(SJisStr::from(&format!(
            r#"<html><!-- 2ch_X:error -->
<head>
    <meta http-equiv="Content-Type" content="text/html; charset=x-sjis">
    <title>ＥＲＲＯＲ</title>
</head>
<body>
    エラー！<br>
    {e}
</body>
</html>"#
        ) as &str))
        .client_ttl(0)
        .server_ttl(0)
        .content_type(SjisContentType::TextHtml)
        .status_code(status_code);
        let resp = if let Some(token) = edge_token {
            resp.add_set_cookie("edge-token".to_string(), token, Duration::days(365))
        } else {
            resp
        };

        resp.build().into_response()
    }
}

#[derive(Debug, Clone)]
pub enum InsufficientParamType {
    Submit,
    Bbs,
    Subject,
    Key,
    From,
    Mail,
    Body,
}

impl Display for InsufficientParamType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                InsufficientParamType::Submit => "submit",
                InsufficientParamType::Bbs => "bbs",
                InsufficientParamType::Subject => "subject",
                InsufficientParamType::Key => "key",
                InsufficientParamType::From => "FROM",
                InsufficientParamType::Mail => "mail",
                InsufficientParamType::Body => "body",
            }
        )
    }
}

impl From<InsufficientParamType> for BbsCgiError {
    fn from(t: InsufficientParamType) -> Self {
        BbsCgiError::InsufficientClientRequest(t)
    }
}

#[derive(Debug, Clone)]
pub enum InvalidParamType {
    Submit,
    Key,
}

impl Display for InvalidParamType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                InvalidParamType::Key => "key",
                InvalidParamType::Submit => "submit",
            }
        )
    }
}

impl From<InvalidParamType> for BbsCgiError {
    fn from(t: InvalidParamType) -> Self {
        BbsCgiError::InvalidClientRequestParameter(t)
    }
}

#[derive(Debug, Clone)]
pub enum NotFoundParamType {
    Board,
    Thread,
}

impl Display for NotFoundParamType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                NotFoundParamType::Board => "板",
                NotFoundParamType::Thread => "スレッド",
            }
        )
    }
}

impl From<NotFoundParamType> for BbsCgiError {
    fn from(t: NotFoundParamType) -> Self {
        BbsCgiError::NotFound(t)
    }
}
