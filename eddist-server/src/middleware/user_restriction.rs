use axum::{
    extract::{Request, State},
    http::{Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use eddist_core::domain::user_restriction::{RestrictionTarget, UserRestrictionRule};

use crate::{
    AppState,
    services::{
        AppService,
        server_settings_cache::{ServerSettingKey, get_server_setting_bool},
        user_restriction_service::{UserRestrictionCheckInput, UserRestrictionCheckOutput},
    },
    utils::{get_asn_num, get_origin_ip, get_ua},
};

pub async fn user_restriction_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    // Check if this is a route we want to restrict
    let path = request.uri().path();
    let authentication_closed = path == "/auth-code"
        && get_server_setting_bool(ServerSettingKey::CloseNewAuthentication).await;
    let target = match restriction_action(request.method(), path, authentication_closed) {
        RestrictionAction::CloseAuthentication => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                "現在、新規認証の受付を停止しています。認証済みの方は引き続き書き込みできます。",
            )
                .into_response();
        }
        RestrictionAction::Check(target) => target,
        RestrictionAction::Skip => return next.run(request).await,
    };

    let headers = request.headers();
    let (Some(ip), Some(asn), Some(ua)) = (
        get_origin_ip(headers),
        get_asn_num(headers),
        get_ua(headers),
    ) else {
        return (StatusCode::FORBIDDEN, "Access denied").into_response();
    };

    let restriction_service = state.get_container().user_restriction();

    let check_input = UserRestrictionCheckInput {
        ip: ip.to_string(),
        asn,
        user_agent: ua.to_string(),
        target,
    };

    match restriction_service.execute(check_input).await {
        Ok(UserRestrictionCheckOutput { matching_rule }) => {
            if let Some(UserRestrictionRule {
                name,
                rule_type,
                rule_value,
                ..
            }) = matching_rule
            {
                tracing::warn!(
                    "Request blocked by user restriction filter: IP={ip}, ASN={asn}, UA={ua}, path={path}, target={target:?}; rule={name}, {rule_type}, {rule_value}"
                );
                return (StatusCode::FORBIDDEN, "Access denied").into_response();
            }
        }
        Err(e) => tracing::error!("Error checking user restrictions: {}", e),
    }

    next.run(request).await
}

#[derive(Debug, PartialEq, Eq)]
enum RestrictionAction {
    Skip,
    CloseAuthentication,
    Check(RestrictionTarget),
}

fn restriction_action(
    method: &Method,
    path: &str,
    authentication_closed: bool,
) -> RestrictionAction {
    match (method, path) {
        (&Method::GET | &Method::POST, "/auth-code") if authentication_closed => {
            RestrictionAction::CloseAuthentication
        }
        (&Method::POST, "/auth-code") => {
            RestrictionAction::Check(RestrictionTarget::Authentication)
        }
        (&Method::POST, "/test/bbs.cgi") => RestrictionAction::Check(RestrictionTarget::Posting),
        _ => RestrictionAction::Skip,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closing_new_authentication_preserves_posting_and_reauthentication() {
        for closed in [false, true] {
            assert_eq!(
                restriction_action(&Method::POST, "/test/bbs.cgi", closed),
                RestrictionAction::Check(RestrictionTarget::Posting)
            );
            for path in ["/re-auth", "/user/login", "/user/auth/callback"] {
                assert_eq!(
                    restriction_action(&Method::POST, path, closed),
                    RestrictionAction::Skip
                );
                assert_eq!(
                    restriction_action(&Method::GET, path, closed),
                    RestrictionAction::Skip
                );
            }
        }
        assert_eq!(
            restriction_action(&Method::GET, "/auth-code", false),
            RestrictionAction::Skip
        );
        assert_eq!(
            restriction_action(&Method::POST, "/auth-code", false),
            RestrictionAction::Check(RestrictionTarget::Authentication)
        );
        for method in [Method::GET, Method::POST] {
            assert_eq!(
                restriction_action(&method, "/auth-code", true),
                RestrictionAction::CloseAuthentication
            );
        }
    }
}
