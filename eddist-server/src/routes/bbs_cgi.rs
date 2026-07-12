use axum::{
    body::Body,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_extra::extract::CookieJar;
use eddist_core::domain::{board::validate_board_key, sjis_str::SJisStr};
use http::HeaderMap;
use jsonwebtoken::EncodingKey;

use crate::{
    AppState,
    error::{BbsCgiError, InsufficientParamType, InvalidParamType},
    services::{
        AppService, BbsCgiService,
        bind_token_to_user_service::BindTokenToUserServiceInput,
        res_creation_service::{ResCreationServiceInput, ResCreationServiceOutput},
        server_settings_cache::{ServerSettingKey, get_server_setting_bool},
        stats_counter::{increment_board_response_delta, increment_board_thread_delta},
        thread_creation_service::{ThreadCreationServiceInput, ThreadCreationServiceOutput},
    },
    shiftjis::{SJisResponseBuilder, SjisContentType, shift_jis_url_encodeded_body_to_vec},
    utils::{get_asn_num, get_origin_ip, get_tinker, get_ua},
};

pub async fn post_bbs_cgi(
    headers: HeaderMap,
    jar: CookieJar,
    State(state): State<AppState>,
    body: String,
) -> Response {
    let Ok(form) = shift_jis_url_encodeded_body_to_vec(&body) else {
        return BbsCgiError::from(InvalidParamType::Body).into_response();
    };
    let is_thread = {
        let Some(submit) = form.get("submit") else {
            return BbsCgiError::from(InsufficientParamType::Submit).into_response();
        };

        match submit as &str {
            "書き込む" => false,
            "新規スレッド作成" => true,
            _ => return BbsCgiError::from(InvalidParamType::Submit).into_response(),
        }
    };

    let (Some(origin_ip), Some(ua), Some(asn_num)) = (
        get_origin_ip(&headers),
        get_ua(&headers),
        get_asn_num(&headers),
    ) else {
        return (StatusCode::FORBIDDEN, "Access denied").into_response();
    };
    // A missing tinker-token cookie is normal (new/anonymous user), but a cookie
    // that is present yet fails to decode is a malformed request - reject it rather
    // than silently treating the client as if it had no tinker.
    let tinker = match jar.get("tinker-token") {
        Some(cookie) => match get_tinker(cookie.value(), state.tinker_secret()) {
            Some(tinker) => Some(tinker),
            None => return (StatusCode::BAD_REQUEST, "invalid tinker-token").into_response(),
        },
        None => None,
    };
    let user_sid = jar.get("user-sid").map(|x| x.value().to_string());
    let edge_token = jar.get("edge-token").map(|x| x.value().to_string());

    let Some(board_key) = form.get("bbs").map(|x| x.to_string()) else {
        return BbsCgiError::from(InsufficientParamType::Bbs).into_response();
    };
    let Some(name) = form.get("FROM").map(|x| x.to_string()) else {
        return BbsCgiError::from(InsufficientParamType::From).into_response();
    };
    let Some(mail) = form.get("mail").map(|x| x.to_string()) else {
        return BbsCgiError::from(InsufficientParamType::Mail).into_response();
    };
    let Some(body) = form.get("MESSAGE").map(|x| x.to_string()) else {
        return BbsCgiError::from(InsufficientParamType::Body).into_response();
    };
    if validate_board_key(&board_key).is_err() {
        return Response::builder().status(404).body(Body::empty()).unwrap();
    }

    fn on_error(e: BbsCgiError, is_thread: bool) -> Response {
        if matches!(e, BbsCgiError::Other(_)) {
            if is_thread {
                log::error!("thread_creation_error, error = {e:?}");
            } else {
                log::error!("res_creation_error, error = {e:?}");
            }
        }
        let is_cookie_reset = matches!(e, BbsCgiError::InvalidAuthedToken);
        let mut resp = e.into_response();
        if is_cookie_reset {
            resp.headers_mut().append(
                "Set-Cookie",
                "edge-token = ; Max-Age = 0; Path = /; "
                    .to_string()
                    .parse()
                    .unwrap(),
            );
            resp.headers_mut().append(
                "Set-Cookie",
                "tinker = ; Max-Age = 0; Path = /; "
                    .to_string()
                    .parse()
                    .unwrap(),
            );
        }
        resp
    }

    let require_user_registration =
        get_server_setting_bool(ServerSettingKey::RequireIdpLinking).await;

    let (tinker, res_order, authed_token_id, is_authed_token_bound) = if is_thread {
        let Some(title) = form.get("subject").map(|x| x.to_string()) else {
            return BbsCgiError::from(InsufficientParamType::Subject).into_response();
        };

        let svc = state.services.thread_creation();
        let (tinker, authed_token_id, is_authed_token_bound) = match svc
            .execute(ThreadCreationServiceInput {
                board_key: board_key.clone(),
                title,
                authed_token: edge_token,
                name,
                mail,
                body,
                tinker,
                ip_addr: origin_ip.to_string(),
                user_agent: ua.to_string(),
                asn_num,
                require_user_registration,
            })
            .await
        {
            Ok(ThreadCreationServiceOutput {
                tinker,
                authed_token_id,
                is_authed_token_bound,
            }) => {
                increment_board_thread_delta(&board_key);
                (tinker, authed_token_id, is_authed_token_bound)
            }
            Err(e) => {
                return on_error(e, true);
            }
        };
        (tinker, None, authed_token_id, is_authed_token_bound)
    } else {
        let Some(thread_number) = form.get("key").map(|x| x.to_string()) else {
            return BbsCgiError::from(InsufficientParamType::Key).into_response();
        };
        let Ok(thread_number) = thread_number.parse::<u64>() else {
            return BbsCgiError::from(InvalidParamType::Key).into_response();
        };

        let svc = state.services.res_creation();
        match svc
            .execute(ResCreationServiceInput {
                board_key: board_key.clone(),
                thread_number,
                authed_token_cookie: edge_token,
                name,
                mail,
                body,
                tinker,
                ip_addr: origin_ip.to_string(),
                user_agent: ua.to_string(),
                asn_num,
                require_user_registration,
            })
            .await
        {
            Ok(ResCreationServiceOutput {
                tinker,
                res_order,
                authed_token_id,
                is_authed_token_bound,
            }) => {
                increment_board_response_delta(&board_key);
                (tinker, res_order, authed_token_id, is_authed_token_bound)
            }
            Err(e) => {
                return on_error(e, false);
            }
        }
    };

    // Fire-and-forget: bind the token to the user if logged in and not yet bound
    if !is_authed_token_bound && let Some(sid) = user_sid {
        let bind_svc = state.services.bind_token_to_user().clone();
        tokio::spawn(async move {
            let _ = bind_svc
                .execute(BindTokenToUserServiceInput {
                    user_sid: sid,
                    authed_token_id,
                })
                .await;
        });
    }

    let mut response = SJisResponseBuilder::new(SJisStr::from(
        r#"<html><!-- 2ch_X:true -->
<head>
    <meta http-equiv="Content-Type" content="text/html; charset=x-sjis">
    <title>書きこみました</title>
</head>
<body>書きこみました</body>
</html>"#,
    ))
    .content_type(SjisContentType::TextHtml)
    .add_set_cookie(
        "tinker-token".to_string(),
        jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &tinker,
            &EncodingKey::from_base64_secret(state.tinker_secret()).unwrap(),
        )
        .unwrap(),
        time::Duration::days(365),
    )
    .add_set_cookie(
        "edge-token".to_string(),
        tinker.authed_token().to_string(),
        time::Duration::days(365),
    );

    if let Some(order) = res_order {
        response = response.add_header("x-resnum".to_string(), order.to_string());
    }

    response.build().into_response()
}
