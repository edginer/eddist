use utoipa::OpenApi;

use crate::{
    auth::__path_post_native_session,
    models::*,
    repository::{
        admin_archive_repository::{
            ArchivedAdminRes, ArchivedAdminThread, ArchivedRes, ArchivedResUpdate, ArchivedThread,
        },
        notice_repository::{CreateNoticeInput, UpdateNoticeInput},
    },
    routes::{archives, auth_tokens, boards, moderation, notices, threads, users},
};

#[derive(OpenApi)]
#[openapi(
    paths(
        // Board routes
        boards::get_boards,
        boards::get_board,
        boards::get_board_info,
        boards::create_board,
        boards::edit_board,

        // Thread routes
        threads::get_threads,
        threads::get_thread,
        threads::get_responses,
        threads::update_response,
        threads::threads_compaction,

        // Archive routes
        archives::get_archived_threads,
        archives::get_archived_thread,
        archives::get_archived_responses,
        archives::get_dat_archived_thread,
        archives::get_admin_dat_archived_thread,
        archives::update_archived_res,
        archives::delete_archived_res,
        archives::delete_archived_thread,

        // Auth token routes
        auth_tokens::get_authed_token,
        auth_tokens::delete_authed_token,

        // Moderation routes
        moderation::get_ng_words,
        moderation::create_ng_word,
        moderation::update_ng_word,
        moderation::delete_ng_word,
        moderation::get_caps,
        moderation::create_cap,
        moderation::update_cap,
        moderation::delete_cap,
        moderation::get_restriction_rules,
        moderation::create_restriction_rule,
        moderation::get_restriction_rule,
        moderation::update_restriction_rule,
        moderation::delete_restriction_rule,

        // User routes
        users::search_users,
        users::update_user_status,

        // Notice routes
        notices::get_notices,
        notices::get_notice,
        notices::create_notice,
        notices::update_notice,
        notices::delete_notice,

        // Auth routes
        post_native_session,
    ),
    components(schemas(
        // Core models
        Board,
        BoardInfo,
        CreateBoardInput,
        EditBoardInput,
        Thread,
        ThreadCompactionInput,
        Res,
        ClientInfo,
        Tinker,
        UpdateResInput,

        // Archive models
        ArchivedThread,
        ArchivedAdminThread,
        ArchivedRes,
        ArchivedAdminRes,
        ArchivedResUpdate,

        // Auth models
        AuthedToken,
        DeleteAuthedTokenInput,
        NativeSessionRequest,
        NativeSessionResponse,
        NativeUserInfo,

        // Moderation models
        NgWord,
        CreationNgWordInput,
        UpdateNgWordInput,
        Cap,
        CreationCapInput,
        UpdateCapInput,
        CreateRestrictionRuleRequest,
        UpdateRestrictionRuleRequest,
        UserRestrictionRuleSchema,
        RestrictionRuleTypeSchema,

        // User models
        User,
        UserIdpBinding,
        UserStatusUpdateInput,
        UserSearchQuery,

        // Notice models
        Notice,
        CreateNoticeInput,
        UpdateNoticeInput,
    ))
)]
pub struct ApiDoc;
