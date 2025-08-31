use std::fmt::Display;

use axum::response::{IntoResponse, Response};
use eddist_core::domain::sjis_str::SJisStr;
use hyper::StatusCode;
use time::Duration;

use crate::{
    external::captcha_like_client::CaptchaLikeError, SJisResponseBuilder, SjisContentType,
};

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

    #[error(
        "認証コード'{auth_code}'を用いて、以下のURLから認証を行ってください \n {base_url}/auth-code"
    )]
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

    #[error("Lv2未満のユーザーは画像URLを投稿できません")]
    ImageUrlBelowLv2,

    #[error("{0}が長すぎます")]
    ContentLengthExceeded(ContentLengthExceededParamType),

    #[error("{0}が空です")]
    ContentEmpty(ContentEmptyParamType),

    #[error(
        "短期間に書き込みすぎです (Lv{tinker_level}は{span_sec}秒以内に1回書き込むことができます)"
    )]
    TooManyCreatingRes { tinker_level: u32, span_sec: i32 },

    #[error(
        "短期間にスレ立てすぎです (Lv{tinker_level}は{span_sec}秒以内に1回スレを立てることができます)"
    )]
    TooManyCreatingThread { tinker_level: u32, span_sec: i32 },

    #[error("短期間にスレ立てすぎです")]
    TooManyCreatingThreadWithoutTinker,

    #[error("初回書き込み時にはスレッドを立てることができません")]
    TmpCanNotCreateThread,

    #[error("この板は現在読み込み専用です")]
    ReadOnlyBoard,

    #[error("以下のURLを利用してユーザー登録を行ってください \n {url}")]
    UserRegTempUrl { url: String },

    #[error("この端末は既にユーザー登録されています")]
    UserAlreadyRegistered,

    #[error("ユーザー登録の試行回数が多すぎます")]
    TooManyUserCreationAttempt,

    #[error("このブラウザではメール欄にトークンを入力しての認証はできません")]
    EmailAuthenticatedUnsupportedUserAgent,

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
            BbsCgiError::ImageUrlBelowLv2 => StatusCode::OK,
            BbsCgiError::ContentLengthExceeded(_) => StatusCode::OK,
            BbsCgiError::ContentEmpty(_) => StatusCode::OK,
            BbsCgiError::TooManyCreatingRes { .. } => StatusCode::OK,
            BbsCgiError::TooManyCreatingThread { .. } => StatusCode::OK,
            BbsCgiError::TooManyCreatingThreadWithoutTinker => StatusCode::OK,
            BbsCgiError::TmpCanNotCreateThread => StatusCode::OK,
            BbsCgiError::ReadOnlyBoard => StatusCode::OK,
            BbsCgiError::UserRegTempUrl { .. } => StatusCode::OK,
            BbsCgiError::UserAlreadyRegistered => StatusCode::OK,
            BbsCgiError::TooManyUserCreationAttempt => StatusCode::OK,
            BbsCgiError::EmailAuthenticatedUnsupportedUserAgent => StatusCode::OK,
            BbsCgiError::Other(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn error_tag(&self) -> &'static str {
        match self {
            BbsCgiError::InsufficientClientRequest(_) => "InsufficientClientRequest",
            BbsCgiError::InvalidClientRequestParameter(_) => "InvalidClientRequestParameter",
            BbsCgiError::NotFound(_) => "NotFound",
            BbsCgiError::InactiveThread => "InactiveThread",
            BbsCgiError::SameTimeThreadCration => "SameTimeThreadCration",
            BbsCgiError::Unauthenticated { .. } => "Unauthenticated",
            BbsCgiError::InvalidAuthedToken => "InvalidAuthedToken",
            BbsCgiError::RevokedAuthedToken => "RevokedAuthedToken",
            BbsCgiError::NgWordDetected => "NgWordDetected",
            BbsCgiError::ImageUrlBelowLv2 => "ImageUrlBelowLv2",
            BbsCgiError::ContentLengthExceeded(_) => "ContentLengthExceeded",
            BbsCgiError::ContentEmpty(_) => "ContentEmpty",
            BbsCgiError::TooManyCreatingRes { .. } => "TooManyCreatingRes",
            BbsCgiError::TooManyCreatingThread { .. } => "TooManyCreatingThread",
            BbsCgiError::TooManyCreatingThreadWithoutTinker => "TooManyCreatingThreadWithoutTinker",
            BbsCgiError::TmpCanNotCreateThread => "TmpCanNotCreateThread",
            BbsCgiError::ReadOnlyBoard => "ReadOnlyBoard",
            BbsCgiError::UserRegTempUrl { .. } => "UserRegTempUrl",
            BbsCgiError::UserAlreadyRegistered => "UserAlreadyRegistered",
            BbsCgiError::TooManyUserCreationAttempt => "TooManyUserCreationAttempt",
            BbsCgiError::EmailAuthenticatedUnsupportedUserAgent => {
                "EmailAuthenticatedUnsupportedUserAgent"
            }
            BbsCgiError::Other(_) => "InternalError",
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

        let error_code = self.error_tag();
        let status_code = self.status_code();
        let e = match self {
            BbsCgiError::Other(_) => "内部エラーが発生しました".to_string(),
            e => e.to_string(),
        };

        let resp = SJisResponseBuilder::new(SJisStr::from(&format!(
            r#"<html><!-- 2ch_X:error -->
<head>
    <meta http-equiv="Content-Type" content="text/html; charset=x-sjis">
    <meta name="error_code" content="E-{error_code}">
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

#[derive(Debug, Clone)]
pub enum ContentLengthExceededParamType {
    Name,
    Mail,
    Body,
    ThreadName,
    BodyLines,
}

impl Display for ContentLengthExceededParamType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ContentLengthExceededParamType::Name => "名前",
                ContentLengthExceededParamType::Mail => "メール",
                ContentLengthExceededParamType::Body => "本文",
                ContentLengthExceededParamType::ThreadName => "スレッド名",
                ContentLengthExceededParamType::BodyLines => "本文の行数",
            }
        )
    }
}

#[derive(Debug, Clone)]
pub enum ContentEmptyParamType {
    ThreadName,
}

impl Display for ContentEmptyParamType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ContentEmptyParamType::ThreadName => "スレッド名",
            }
        )
    }
}

#[derive(thiserror::Error, Debug)]
pub enum BbsPostAuthWithCodeError {
    #[error("書き込み時のIPアドレスと異なるか、入力した6桁の数字が誤りです")]
    FailedToFindAuthedToken,
    #[error("認証コードの有効期限が切れています。再度認証してください")]
    ExpiredActivationCode,
    #[error("認証に失敗しました。時間をおいてから再度認証してください")]
    AuthCodeCollision,
    #[error("認証トークンの発行制限中です。1時間後に再度お試しください。")]
    RateLimited,
    #[error(transparent)]
    CaptchaError(#[from] CaptchaLikeError),
}
