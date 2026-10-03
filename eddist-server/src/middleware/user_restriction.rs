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
    let Some(target) = restriction_target(request.method(), path) else {
        return next.run(request).await;
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

fn restriction_target(method: &Method, path: &str) -> Option<RestrictionTarget> {
    match (method, path) {
        (&Method::POST, "/auth-code") => Some(RestrictionTarget::Authentication),
        (&Method::POST, "/test/bbs.cgi") => Some(RestrictionTarget::Posting),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restriction_targets_only_authentication_and_posting() {
        assert_eq!(
            restriction_target(&Method::POST, "/auth-code"),
            Some(RestrictionTarget::Authentication)
        );
        assert_eq!(
            restriction_target(&Method::POST, "/test/bbs.cgi"),
            Some(RestrictionTarget::Posting)
        );
        assert_eq!(restriction_target(&Method::GET, "/auth-code"), None);
        for path in ["/re-auth", "/user/login", "/user/auth/callback"] {
            assert_eq!(restriction_target(&Method::POST, path), None);
            assert_eq!(restriction_target(&Method::GET, path), None);
        }
    }
}
