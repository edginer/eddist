/// Generated protobuf types for all bbs:event:* channels.
/// The canonical schema is at eddist-core/proto/bbs_events.proto.
pub mod events {
    include!(concat!(env!("OUT_DIR"), "/eddist.events.rs"));
}

use prost::Message;
use prost_types::Timestamp;
use uuid::Uuid;

use crate::domain::{
    client_info::ClientInfo,
    metadent::MetadentType,
    pubsub_repository::{
        AuthTokenInitiated, AuthTokenRequested, AuthTokenRevoked, AuthTokenSucceeded, CreatingRes,
        CreatingThread, ModerationResult,
    },
    tinker::Tinker,
};

// ── helpers ───────────────────────────────────────────────────────────────────

fn uuid_to_bytes(id: Uuid) -> Vec<u8> {
    id.as_bytes().to_vec()
}

fn bytes_to_uuid(bytes: &[u8]) -> Uuid {
    Uuid::from_slice(bytes).expect("invalid UUID bytes in protobuf message")
}

fn dt_to_ts(dt: chrono::DateTime<chrono::Utc>) -> Timestamp {
    Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    }
}

fn ts_to_dt(ts: Option<Timestamp>) -> chrono::DateTime<chrono::Utc> {
    ts.and_then(|t| chrono::DateTime::from_timestamp(t.seconds, t.nanos as u32))
        .unwrap_or_default()
}

fn metadent_to_i32(m: MetadentType) -> i32 {
    match m {
        MetadentType::None => 0,
        MetadentType::Verbose => 1,
        MetadentType::VVerbose => 2,
        MetadentType::VVVerbose => 3,
    }
}

fn i32_to_metadent(i: i32) -> MetadentType {
    match i {
        1 => MetadentType::Verbose,
        2 => MetadentType::VVerbose,
        3 => MetadentType::VVVerbose,
        _ => MetadentType::None,
    }
}

// ── Tinker ────────────────────────────────────────────────────────────────────

impl From<&Tinker> for events::Tinker {
    fn from(t: &Tinker) -> Self {
        Self {
            authed_token: t.authed_token().to_string(),
            wrote_count: t.wrote_count(),
            created_thread_count: t.created_thread_count(),
            level: t.level(),
            last_level_up_at: t.last_level_up_at(),
            last_wrote_at: t.last_wrote_at(),
            last_created_thread_at: t.last_created_thread_at(),
        }
    }
}

impl From<events::Tinker> for Tinker {
    fn from(p: events::Tinker) -> Self {
        Tinker::from_parts(
            p.authed_token,
            p.wrote_count,
            p.created_thread_count,
            p.level,
            p.last_level_up_at,
            p.last_wrote_at,
            p.last_created_thread_at,
        )
    }
}

// ── ClientInfo ────────────────────────────────────────────────────────────────

impl From<&ClientInfo> for events::ClientInfo {
    fn from(c: &ClientInfo) -> Self {
        Self {
            user_agent: c.user_agent.clone(),
            asn_num: c.asn_num,
            ip_addr: c.ip_addr.clone(),
            tinker: c.tinker.as_deref().map(events::Tinker::from),
        }
    }
}

impl From<events::ClientInfo> for ClientInfo {
    fn from(p: events::ClientInfo) -> Self {
        Self {
            user_agent: p.user_agent,
            asn_num: p.asn_num,
            ip_addr: p.ip_addr,
            tinker: p.tinker.map(|t| Box::new(Tinker::from(t))),
        }
    }
}

// ── ModerationResult ──────────────────────────────────────────────────────────

impl From<&ModerationResult> for events::ModerationResult {
    fn from(m: &ModerationResult) -> Self {
        Self {
            flagged: m.flagged,
            categories: m.categories.to_string(),
            category_scores: m.category_scores.to_string(),
        }
    }
}

impl From<events::ModerationResult> for ModerationResult {
    fn from(p: events::ModerationResult) -> Self {
        Self {
            flagged: p.flagged,
            categories: serde_json::from_str(&p.categories).unwrap_or(serde_json::Value::Null),
            category_scores: serde_json::from_str(&p.category_scores)
                .unwrap_or(serde_json::Value::Null),
        }
    }
}

// ── CreatingThread ────────────────────────────────────────────────────────────

impl From<&CreatingThread> for events::CreatingThread {
    fn from(e: &CreatingThread) -> Self {
        Self {
            thread_id: uuid_to_bytes(e.thread_id),
            response_id: uuid_to_bytes(e.response_id),
            title: e.title.clone(),
            unix_time: e.unix_time,
            body: e.body.clone(),
            name: e.name.clone(),
            mail: e.mail.clone(),
            created_at: Some(dt_to_ts(e.created_at)),
            author_ch5id: e.author_ch5id.clone(),
            authed_token_id: uuid_to_bytes(e.authed_token_id),
            ip_addr: e.ip_addr.clone(),
            board_id: uuid_to_bytes(e.board_id),
            metadent: metadent_to_i32(e.metadent),
            client_info: Some(events::ClientInfo::from(&e.client_info)),
            moderation_result: e
                .moderation_result
                .as_ref()
                .map(events::ModerationResult::from),
        }
    }
}

impl TryFrom<events::CreatingThread> for CreatingThread {
    type Error = prost::DecodeError;

    fn try_from(p: events::CreatingThread) -> Result<Self, Self::Error> {
        Ok(Self {
            thread_id: bytes_to_uuid(&p.thread_id),
            response_id: bytes_to_uuid(&p.response_id),
            title: p.title,
            unix_time: p.unix_time,
            body: p.body,
            name: p.name,
            mail: p.mail,
            created_at: ts_to_dt(p.created_at),
            author_ch5id: p.author_ch5id,
            authed_token_id: bytes_to_uuid(&p.authed_token_id),
            ip_addr: p.ip_addr,
            board_id: bytes_to_uuid(&p.board_id),
            metadent: i32_to_metadent(p.metadent),
            client_info: p.client_info.map(ClientInfo::from).unwrap_or_default(),
            moderation_result: p.moderation_result.map(ModerationResult::from),
        })
    }
}

// ── CreatingRes ───────────────────────────────────────────────────────────────

impl From<&CreatingRes> for events::CreatingRes {
    fn from(e: &CreatingRes) -> Self {
        Self {
            id: uuid_to_bytes(e.id),
            created_at: Some(dt_to_ts(e.created_at)),
            body: e.body.clone(),
            name: e.name.clone(),
            mail: e.mail.clone(),
            author_ch5id: e.author_ch5id.clone(),
            authed_token_id: uuid_to_bytes(e.authed_token_id),
            ip_addr: e.ip_addr.clone(),
            thread_id: uuid_to_bytes(e.thread_id),
            board_id: uuid_to_bytes(e.board_id),
            client_info: Some(events::ClientInfo::from(&e.client_info)),
            res_order: e.res_order,
            is_sage: e.is_sage,
            moderation_result: e
                .moderation_result
                .as_ref()
                .map(events::ModerationResult::from),
        }
    }
}

impl TryFrom<events::CreatingRes> for CreatingRes {
    type Error = prost::DecodeError;

    fn try_from(p: events::CreatingRes) -> Result<Self, Self::Error> {
        Ok(Self {
            id: bytes_to_uuid(&p.id),
            created_at: ts_to_dt(p.created_at),
            body: p.body,
            name: p.name,
            mail: p.mail,
            author_ch5id: p.author_ch5id,
            authed_token_id: bytes_to_uuid(&p.authed_token_id),
            ip_addr: p.ip_addr,
            thread_id: bytes_to_uuid(&p.thread_id),
            board_id: bytes_to_uuid(&p.board_id),
            client_info: p.client_info.map(ClientInfo::from).unwrap_or_default(),
            res_order: p.res_order,
            is_sage: p.is_sage,
            moderation_result: p.moderation_result.map(ModerationResult::from),
        })
    }
}

// ── AuthToken events ──────────────────────────────────────────────────────────

impl From<&AuthTokenInitiated> for events::AuthTokenInitiated {
    fn from(e: &AuthTokenInitiated) -> Self {
        Self {
            authed_token_id: uuid_to_bytes(e.authed_token_id),
            origin_ip: e.origin_ip.clone(),
            user_agent: e.user_agent.clone(),
            asn_num: e.asn_num,
        }
    }
}

impl From<events::AuthTokenInitiated> for AuthTokenInitiated {
    fn from(p: events::AuthTokenInitiated) -> Self {
        Self {
            authed_token_id: bytes_to_uuid(&p.authed_token_id),
            origin_ip: p.origin_ip,
            user_agent: p.user_agent,
            asn_num: p.asn_num,
        }
    }
}

impl From<&AuthTokenRequested> for events::AuthTokenRequested {
    fn from(e: &AuthTokenRequested) -> Self {
        Self {
            authed_token_id: e.authed_token_id.map(uuid_to_bytes),
            origin_ip: e.origin_ip.clone(),
            user_agent: e.user_agent.clone(),
            asn_num: e.asn_num,
            auth_code: e.auth_code.clone(),
        }
    }
}

impl From<events::AuthTokenRequested> for AuthTokenRequested {
    fn from(p: events::AuthTokenRequested) -> Self {
        Self {
            authed_token_id: p.authed_token_id.as_deref().map(bytes_to_uuid),
            origin_ip: p.origin_ip,
            user_agent: p.user_agent,
            asn_num: p.asn_num,
            auth_code: p.auth_code,
        }
    }
}

impl From<&AuthTokenSucceeded> for events::AuthTokenSucceeded {
    fn from(e: &AuthTokenSucceeded) -> Self {
        Self {
            authed_token_id: uuid_to_bytes(e.authed_token_id),
            origin_ip: e.origin_ip.clone(),
            user_agent: e.user_agent.clone(),
            asn_num: e.asn_num,
            authed_at: Some(dt_to_ts(e.authed_at)),
            additional_info: e.additional_info.as_ref().map(|v| v.to_string()),
        }
    }
}

impl From<events::AuthTokenSucceeded> for AuthTokenSucceeded {
    fn from(p: events::AuthTokenSucceeded) -> Self {
        Self {
            authed_token_id: bytes_to_uuid(&p.authed_token_id),
            origin_ip: p.origin_ip,
            user_agent: p.user_agent,
            asn_num: p.asn_num,
            authed_at: ts_to_dt(p.authed_at),
            additional_info: p
                .additional_info
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok()),
        }
    }
}

impl From<&AuthTokenRevoked> for events::AuthTokenRevoked {
    fn from(e: &AuthTokenRevoked) -> Self {
        Self {
            authed_token_id: uuid_to_bytes(e.authed_token_id),
        }
    }
}

impl From<events::AuthTokenRevoked> for AuthTokenRevoked {
    fn from(p: events::AuthTokenRevoked) -> Self {
        Self {
            authed_token_id: bytes_to_uuid(&p.authed_token_id),
        }
    }
}

// ── Public encode/decode helpers ──────────────────────────────────────────────

pub fn encode_creating_thread(e: &CreatingThread) -> Vec<u8> {
    events::CreatingThread::from(e).encode_to_vec()
}

pub fn decode_creating_thread(bytes: &[u8]) -> Result<CreatingThread, prost::DecodeError> {
    events::CreatingThread::decode(bytes)?.try_into()
}

pub fn encode_creating_res(e: &CreatingRes) -> Vec<u8> {
    events::CreatingRes::from(e).encode_to_vec()
}

pub fn decode_creating_res(bytes: &[u8]) -> Result<CreatingRes, prost::DecodeError> {
    events::CreatingRes::decode(bytes)?.try_into()
}

pub fn encode_auth_token_initiated(e: &AuthTokenInitiated) -> Vec<u8> {
    events::AuthTokenInitiated::from(e).encode_to_vec()
}

pub fn decode_auth_token_initiated(bytes: &[u8]) -> Result<AuthTokenInitiated, prost::DecodeError> {
    events::AuthTokenInitiated::decode(bytes).map(Into::into)
}

pub fn encode_auth_token_requested(e: &AuthTokenRequested) -> Vec<u8> {
    events::AuthTokenRequested::from(e).encode_to_vec()
}

pub fn decode_auth_token_requested(bytes: &[u8]) -> Result<AuthTokenRequested, prost::DecodeError> {
    events::AuthTokenRequested::decode(bytes).map(Into::into)
}

pub fn encode_auth_token_succeeded(e: &AuthTokenSucceeded) -> Vec<u8> {
    events::AuthTokenSucceeded::from(e).encode_to_vec()
}

pub fn decode_auth_token_succeeded(bytes: &[u8]) -> Result<AuthTokenSucceeded, prost::DecodeError> {
    events::AuthTokenSucceeded::decode(bytes).map(Into::into)
}

pub fn encode_auth_token_revoked(e: &AuthTokenRevoked) -> Vec<u8> {
    events::AuthTokenRevoked::from(e).encode_to_vec()
}

pub fn decode_auth_token_revoked(bytes: &[u8]) -> Result<AuthTokenRevoked, prost::DecodeError> {
    events::AuthTokenRevoked::decode(bytes).map(Into::into)
}
