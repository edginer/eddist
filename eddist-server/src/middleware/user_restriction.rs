use axum::{
    extract::{Request, State},
    http::{Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use eddist_core::domain::user_restriction::UserRestrictionRule;

use crate::{
    services::{
        user_restriction_service::{UserRestrictionCheckInput, UserRestrictionCheckOutput},
        AppService,
    },
    utils::{get_asn_num, get_origin_ip, get_ua},
    AppState,
};

pub async fn user_restriction_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    // Check if this is a route we want to restrict
    let path = request.uri().path();
    let should_check = path.starts_with("/test/bbs.cgi")
        || (path.starts_with("/auth-code") && request.method() == Method::POST);

    if !should_check {
        return next.run(request).await;
    }

    let headers = request.headers();
    let ip = get_origin_ip(headers);
    let asn = get_asn_num(headers);
    let ua = get_ua(headers);

    let restriction_service = state.get_container().user_restriction();

    let check_input = UserRestrictionCheckInput {
        ip: ip.to_string(),
        asn,
        user_agent: ua.to_string(),
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
                    "Request blocked by user restriction filter: IP={ip}, ASN={asn}, UA={ua}, path={path}; rule={name}, {rule_type}, {rule_value}"
                );
                return (StatusCode::FORBIDDEN, "Access denied").into_response();
            }
        }
        Err(e) => tracing::error!("Error checking user restrictions: {}", e),
    }

    next.run(request).await
}
