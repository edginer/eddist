use crate::entity::{
    admin_role, admin_role_scope, admin_user, authed_token, board, board_cap, board_info,
    board_ng_word, cap, captcha_config, daily_stat, idp, ng_word, notice, response, server_setting,
    term, thread, user, user_authed_token, user_idp_binding, user_restriction,
};
use anyhow::Result;
use sea_orm::sea_query::OnConflict;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectOptions, Database, DatabaseConnection,
    EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
};

const BATCH_SIZE: u64 = 500;

fn connect_options(url: &str) -> ConnectOptions {
    ConnectOptions::new(url)
}

async fn insert_no_conflict<E, I>(db: &DatabaseConnection, models: I) -> Result<()>
where
    E: EntityTrait,
    I: IntoIterator<Item = E::ActiveModel>,
{
    E::insert_many(models)
        .on_conflict(OnConflict::new().do_nothing().to_owned())
        .exec(db)
        .await?;
    Ok(())
}

pub async fn run(mysql_url: &str, pg_url: &str) -> Result<()> {
    println!("Connecting to MySQL...");
    let mysql = Database::connect(connect_options(mysql_url)).await?;
    println!("Connecting to PostgreSQL...");
    let pg = Database::connect(connect_options(pg_url)).await?;
    println!("Migrating (archived_responses and archived_threads excluded)\n");

    // Insert authed_tokens first with registered_user_id = NULL (circular FK with users).
    // After users are inserted, fix_registered_user_id() fills it in.
    migrate_authed_tokens(&mysql, &pg).await?;
    migrate_boards(&mysql, &pg).await?;
    migrate_boards_info(&mysql, &pg).await?;
    migrate_idps(&mysql, &pg).await?;
    migrate_users(&mysql, &pg).await?;
    fix_registered_user_id(&mysql, &pg).await?;
    migrate_admin_roles(&mysql, &pg).await?;
    migrate_admin_role_scopes(&mysql, &pg).await?;
    migrate_admin_users(&mysql, &pg).await?;
    migrate_caps(&mysql, &pg).await?;
    migrate_ng_words(&mysql, &pg).await?;
    migrate_threads(&mysql, &pg).await?;
    migrate_responses(&mysql, &pg).await?;
    migrate_boards_caps(&mysql, &pg).await?;
    migrate_boards_ng_words(&mysql, &pg).await?;
    migrate_user_idp_bindings(&mysql, &pg).await?;
    migrate_user_authed_tokens(&mysql, &pg).await?;
    migrate_user_restriction_rules(&mysql, &pg).await?;
    migrate_notices(&mysql, &pg).await?;
    migrate_terms(&mysql, &pg).await?;
    migrate_captcha_configs(&mysql, &pg).await?;
    migrate_server_settings(&mysql, &pg).await?;
    migrate_daily_stats(&mysql, &pg).await?;

    println!("\nDone.");
    Ok(())
}

async fn migrate_authed_tokens(mysql: &DatabaseConnection, pg: &DatabaseConnection) -> Result<()> {
    let mut pages = authed_token::Entity::find()
        .order_by_asc(authed_token::Column::Id)
        .paginate(mysql, BATCH_SIZE);
    let mut total = 0u64;

    while let Some(rows) = pages.fetch_and_next().await? {
        total += rows.len() as u64;
        let models = rows
            .into_iter()
            .map(|row| authed_token::ActiveModel {
                id: Set(row.id),
                token: Set(row.token),
                origin_ip: Set(row.origin_ip),
                reduced_origin_ip: Set(row.reduced_origin_ip),
                writing_ua: Set(row.writing_ua),
                authed_ua: Set(row.authed_ua),
                auth_code: Set(row.auth_code),
                created_at: Set(row.created_at),
                authed_at: Set(row.authed_at),
                validity: Set(row.validity),
                last_wrote_at: Set(row.last_wrote_at),
                asn_num: Set(row.asn_num),
                additional_info: Set(row.additional_info),
                require_user_registration: Set(row.require_user_registration),
                registered_user_id: Set(None),
                require_reauth: Set(row.require_reauth),
                author_id_seed: Set(row.author_id_seed),
            })
            .collect::<Vec<_>>();
        insert_no_conflict::<authed_token::Entity, _>(pg, models).await?;
    }

    println!("authed_tokens:          {total}");
    Ok(())
}

async fn fix_registered_user_id(mysql: &DatabaseConnection, pg: &DatabaseConnection) -> Result<()> {
    let rows = authed_token::Entity::find()
        .filter(authed_token::Column::RegisteredUserId.is_not_null())
        .all(mysql)
        .await?;

    let count = rows.len();
    for row in rows {
        let model = authed_token::ActiveModel {
            id: Set(row.id),
            registered_user_id: Set(row.registered_user_id),
            ..Default::default()
        };
        model.update(pg).await?;
    }
    println!("authed_tokens (reg_user_id fix): {count}");
    Ok(())
}

async fn migrate_boards(mysql: &DatabaseConnection, pg: &DatabaseConnection) -> Result<()> {
    let rows = board::Entity::find().all(mysql).await?;
    let count = rows.len();
    let models = rows
        .into_iter()
        .map(|row| board::ActiveModel {
            id: Set(row.id),
            name: Set(row.name),
            board_key: Set(row.board_key),
            default_name: Set(row.default_name),
        })
        .collect::<Vec<_>>();

    board::Entity::insert_many(models)
        .on_conflict(
            OnConflict::column(board::Column::Id)
                .update_columns([
                    board::Column::Name,
                    board::Column::BoardKey,
                    board::Column::DefaultName,
                ])
                .to_owned(),
        )
        .exec(pg)
        .await?;
    println!("boards:                 {count}");
    Ok(())
}

async fn migrate_boards_info(mysql: &DatabaseConnection, pg: &DatabaseConnection) -> Result<()> {
    let rows = board_info::Entity::find().all(mysql).await?;
    let count = rows.len();
    let models = rows
        .into_iter()
        .map(|row| board_info::ActiveModel {
            id: Set(row.id),
            local_rules: Set(row.local_rules),
            base_thread_creation_span_sec: Set(row.base_thread_creation_span_sec),
            base_response_creation_span_sec: Set(row.base_response_creation_span_sec),
            max_thread_name_byte_length: Set(row.max_thread_name_byte_length),
            max_author_name_byte_length: Set(row.max_author_name_byte_length),
            max_email_byte_length: Set(row.max_email_byte_length),
            max_response_body_byte_length: Set(row.max_response_body_byte_length),
            max_response_body_lines: Set(row.max_response_body_lines),
            threads_archive_cron: Set(row.threads_archive_cron),
            threads_archive_trigger_thread_count: Set(row.threads_archive_trigger_thread_count),
            read_only: Set(row.read_only),
            created_at: Set(row.created_at),
            updated_at: Set(row.updated_at),
            force_metadent_type: Set(row.force_metadent_type),
            enable_1001_message: Set(row.enable_1001_message),
            custom_1001_message: Set(row.custom_1001_message),
        })
        .collect::<Vec<_>>();

    board_info::Entity::insert_many(models)
        .on_conflict(
            OnConflict::column(board_info::Column::Id)
                .update_columns([
                    board_info::Column::LocalRules,
                    board_info::Column::ThreadsArchiveCron,
                    board_info::Column::ThreadsArchiveTriggerThreadCount,
                    board_info::Column::ReadOnly,
                    board_info::Column::UpdatedAt,
                    board_info::Column::ForceMetadentType,
                    board_info::Column::Enable1001Message,
                    board_info::Column::Custom1001Message,
                ])
                .to_owned(),
        )
        .exec(pg)
        .await?;
    println!("boards_info:            {count}");
    Ok(())
}

async fn migrate_idps(mysql: &DatabaseConnection, pg: &DatabaseConnection) -> Result<()> {
    let rows = idp::Entity::find().all(mysql).await?;
    let count = rows.len();
    let models = rows
        .into_iter()
        .map(|row| idp::ActiveModel {
            id: Set(row.id),
            idp_name: Set(row.idp_name),
            idp_display_name: Set(row.idp_display_name),
            idp_logo_svg: Set(row.idp_logo_svg),
            oidc_config_url: Set(row.oidc_config_url),
            client_id: Set(row.client_id),
            client_secret: Set(row.client_secret),
            enabled: Set(row.enabled),
        })
        .collect::<Vec<_>>();
    insert_no_conflict::<idp::Entity, _>(pg, models).await?;
    println!("idps:                   {count}");
    Ok(())
}

async fn migrate_users(mysql: &DatabaseConnection, pg: &DatabaseConnection) -> Result<()> {
    let rows = user::Entity::find().all(mysql).await?;
    let count = rows.len();
    let models = rows
        .into_iter()
        .map(|row| user::ActiveModel {
            id: Set(row.id),
            user_name: Set(row.user_name),
            enabled: Set(row.enabled),
            created_at: Set(row.created_at),
            updated_at: Set(row.updated_at),
        })
        .collect::<Vec<_>>();
    insert_no_conflict::<user::Entity, _>(pg, models).await?;
    println!("users:                  {count}");
    Ok(())
}

async fn migrate_admin_roles(mysql: &DatabaseConnection, pg: &DatabaseConnection) -> Result<()> {
    let rows = admin_role::Entity::find().all(mysql).await?;
    let count = rows.len();
    let models = rows
        .into_iter()
        .map(|row| admin_role::ActiveModel {
            id: Set(row.id),
            role_name: Set(row.role_name),
            role_description: Set(row.role_description),
        })
        .collect::<Vec<_>>();
    insert_no_conflict::<admin_role::Entity, _>(pg, models).await?;
    println!("admin_roles:            {count}");
    Ok(())
}

async fn migrate_admin_role_scopes(
    mysql: &DatabaseConnection,
    pg: &DatabaseConnection,
) -> Result<()> {
    let rows = admin_role_scope::Entity::find().all(mysql).await?;
    let count = rows.len();
    let models = rows
        .into_iter()
        .map(|row| admin_role_scope::ActiveModel {
            id: Set(row.id),
            role_id: Set(row.role_id),
            scope_key: Set(row.scope_key),
        })
        .collect::<Vec<_>>();
    insert_no_conflict::<admin_role_scope::Entity, _>(pg, models).await?;
    println!("admin_role_scopes:      {count}");
    Ok(())
}

async fn migrate_admin_users(mysql: &DatabaseConnection, pg: &DatabaseConnection) -> Result<()> {
    let rows = admin_user::Entity::find().all(mysql).await?;
    let count = rows.len();
    let models = rows
        .into_iter()
        .map(|row| admin_user::ActiveModel {
            id: Set(row.id),
            user_role_id: Set(row.user_role_id),
        })
        .collect::<Vec<_>>();
    insert_no_conflict::<admin_user::Entity, _>(pg, models).await?;
    println!("admin_users:            {count}");
    Ok(())
}

async fn migrate_caps(mysql: &DatabaseConnection, pg: &DatabaseConnection) -> Result<()> {
    let rows = cap::Entity::find().all(mysql).await?;
    let count = rows.len();
    let models = rows
        .into_iter()
        .map(|row| cap::ActiveModel {
            id: Set(row.id),
            name: Set(row.name),
            description: Set(row.description),
            password_hash: Set(row.password_hash),
            created_at: Set(row.created_at),
            updated_at: Set(row.updated_at),
        })
        .collect::<Vec<_>>();
    insert_no_conflict::<cap::Entity, _>(pg, models).await?;
    println!("caps:                   {count}");
    Ok(())
}

async fn migrate_ng_words(mysql: &DatabaseConnection, pg: &DatabaseConnection) -> Result<()> {
    let rows = ng_word::Entity::find().all(mysql).await?;
    let count = rows.len();
    let models = rows
        .into_iter()
        .map(|row| ng_word::ActiveModel {
            id: Set(row.id),
            name: Set(row.name),
            word: Set(row.word),
            created_at: Set(row.created_at),
            updated_at: Set(row.updated_at),
        })
        .collect::<Vec<_>>();
    insert_no_conflict::<ng_word::Entity, _>(pg, models).await?;
    println!("ng_words:               {count}");
    Ok(())
}

async fn migrate_threads(mysql: &DatabaseConnection, pg: &DatabaseConnection) -> Result<()> {
    let rows = thread::Entity::find().all(mysql).await?;
    let count = rows.len();
    let models = rows
        .into_iter()
        .map(|row| thread::ActiveModel {
            id: Set(row.id),
            board_id: Set(row.board_id),
            thread_number: Set(row.thread_number),
            last_modified_at: Set(row.last_modified_at),
            sage_last_modified_at: Set(row.sage_last_modified_at),
            title: Set(row.title),
            authed_token_id: Set(row.authed_token_id),
            metadent: Set(row.metadent),
            response_count: Set(row.response_count),
            no_pool: Set(row.no_pool),
            active: Set(row.active),
            archived: Set(row.archived),
            archive_converted: Set(row.archive_converted),
        })
        .collect::<Vec<_>>();
    insert_no_conflict::<thread::Entity, _>(pg, models).await?;
    println!("threads:                {count}");
    Ok(())
}

async fn migrate_responses(mysql: &DatabaseConnection, pg: &DatabaseConnection) -> Result<()> {
    let mut pages = response::Entity::find()
        .order_by_asc(response::Column::Id)
        .paginate(mysql, BATCH_SIZE);
    let mut total = 0u64;

    while let Some(rows) = pages.fetch_and_next().await? {
        total += rows.len() as u64;
        let models = rows
            .into_iter()
            .map(|row| response::ActiveModel {
                id: Set(row.id),
                author_name: Set(row.author_name),
                mail: Set(row.mail),
                body: Set(row.body),
                created_at: Set(row.created_at),
                author_id: Set(row.author_id),
                ip_addr: Set(row.ip_addr),
                authed_token_id: Set(row.authed_token_id),
                board_id: Set(row.board_id),
                thread_id: Set(row.thread_id),
                is_abone: Set(row.is_abone),
                is_abone_keep_id: Set(row.is_abone_keep_id),
                res_order: Set(row.res_order),
                client_info: Set(row.client_info),
            })
            .collect::<Vec<_>>();
        insert_no_conflict::<response::Entity, _>(pg, models).await?;
    }

    println!("responses:              {total}");
    Ok(())
}

async fn migrate_boards_caps(mysql: &DatabaseConnection, pg: &DatabaseConnection) -> Result<()> {
    let rows = board_cap::Entity::find().all(mysql).await?;
    let count = rows.len();
    let models = rows
        .into_iter()
        .map(|row| board_cap::ActiveModel {
            id: Set(row.id),
            board_id: Set(row.board_id),
            cap_id: Set(row.cap_id),
        })
        .collect::<Vec<_>>();
    insert_no_conflict::<board_cap::Entity, _>(pg, models).await?;
    println!("boards_caps:            {count}");
    Ok(())
}

async fn migrate_boards_ng_words(
    mysql: &DatabaseConnection,
    pg: &DatabaseConnection,
) -> Result<()> {
    let rows = board_ng_word::Entity::find().all(mysql).await?;
    let count = rows.len();
    let models = rows
        .into_iter()
        .map(|row| board_ng_word::ActiveModel {
            id: Set(row.id),
            board_id: Set(row.board_id),
            ng_word_id: Set(row.ng_word_id),
        })
        .collect::<Vec<_>>();
    insert_no_conflict::<board_ng_word::Entity, _>(pg, models).await?;
    println!("boards_ng_words:        {count}");
    Ok(())
}

async fn migrate_user_idp_bindings(
    mysql: &DatabaseConnection,
    pg: &DatabaseConnection,
) -> Result<()> {
    let rows = user_idp_binding::Entity::find().all(mysql).await?;
    let count = rows.len();
    let models = rows
        .into_iter()
        .map(|row| user_idp_binding::ActiveModel {
            id: Set(row.id),
            user_id: Set(row.user_id),
            idp_id: Set(row.idp_id),
            idp_sub: Set(row.idp_sub),
            created_at: Set(row.created_at),
            updated_at: Set(row.updated_at),
        })
        .collect::<Vec<_>>();
    insert_no_conflict::<user_idp_binding::Entity, _>(pg, models).await?;
    println!("user_idp_bindings:      {count}");
    Ok(())
}

async fn migrate_user_authed_tokens(
    mysql: &DatabaseConnection,
    pg: &DatabaseConnection,
) -> Result<()> {
    let rows = user_authed_token::Entity::find().all(mysql).await?;
    let count = rows.len();
    let models = rows
        .into_iter()
        .map(|row| user_authed_token::ActiveModel {
            id: Set(row.id),
            user_id: Set(row.user_id),
            authed_token_id: Set(row.authed_token_id),
            created_at: Set(row.created_at),
            updated_at: Set(row.updated_at),
        })
        .collect::<Vec<_>>();
    insert_no_conflict::<user_authed_token::Entity, _>(pg, models).await?;
    println!("user_authed_tokens:     {count}");
    Ok(())
}

async fn migrate_user_restriction_rules(
    mysql: &DatabaseConnection,
    pg: &DatabaseConnection,
) -> Result<()> {
    let rows = user_restriction::Entity::find().all(mysql).await?;
    let count = rows.len();
    let models = rows
        .into_iter()
        .map(|row| user_restriction::ActiveModel {
            id: Set(row.id),
            name: Set(row.name),
            rule_type: Set(row.rule_type),
            rule_value: Set(row.rule_value),
            expires_at: Set(row.expires_at),
            created_at: Set(row.created_at),
            updated_at: Set(row.updated_at),
            created_by_email: Set(row.created_by_email),
        })
        .collect::<Vec<_>>();
    insert_no_conflict::<user_restriction::Entity, _>(pg, models).await?;
    println!("user_restriction_rules: {count}");
    Ok(())
}

async fn migrate_notices(mysql: &DatabaseConnection, pg: &DatabaseConnection) -> Result<()> {
    let rows = notice::Entity::find().all(mysql).await?;
    let count = rows.len();
    let models = rows
        .into_iter()
        .map(|row| notice::ActiveModel {
            id: Set(row.id),
            slug: Set(row.slug),
            title: Set(row.title),
            content: Set(row.content),
            created_at: Set(row.created_at),
            updated_at: Set(row.updated_at),
            published_at: Set(row.published_at),
            author_email: Set(row.author_email),
            hide_from_list: Set(row.hide_from_list),
        })
        .collect::<Vec<_>>();
    insert_no_conflict::<notice::Entity, _>(pg, models).await?;
    println!("notices:                {count}");
    Ok(())
}

async fn migrate_terms(mysql: &DatabaseConnection, pg: &DatabaseConnection) -> Result<()> {
    let rows = term::Entity::find().all(mysql).await?;
    let count = rows.len();
    let models = rows
        .into_iter()
        .map(|row| term::ActiveModel {
            id: Set(row.id),
            content: Set(row.content),
            created_at: Set(row.created_at),
            updated_at: Set(row.updated_at),
            updated_by: Set(row.updated_by),
        })
        .collect::<Vec<_>>();

    term::Entity::insert_many(models)
        .on_conflict(
            OnConflict::column(term::Column::Id)
                .update_columns([
                    term::Column::Content,
                    term::Column::UpdatedAt,
                    term::Column::UpdatedBy,
                ])
                .to_owned(),
        )
        .exec(pg)
        .await?;
    println!("terms:                  {count}");
    Ok(())
}

async fn migrate_captcha_configs(
    mysql: &DatabaseConnection,
    pg: &DatabaseConnection,
) -> Result<()> {
    let rows = captcha_config::Entity::find().all(mysql).await?;
    let count = rows.len();
    let models = rows
        .into_iter()
        .map(|row| captcha_config::ActiveModel {
            id: Set(row.id),
            name: Set(row.name),
            provider: Set(row.provider),
            site_key: Set(row.site_key),
            secret: Set(row.secret),
            base_url: Set(row.base_url),
            widget_form_field_name: Set(row.widget_form_field_name),
            widget_script_url: Set(row.widget_script_url),
            widget_html: Set(row.widget_html),
            widget_script_handler: Set(row.widget_script_handler),
            capture_fields: Set(row.capture_fields),
            verification: Set(row.verification),
            is_active: Set(row.is_active),
            display_order: Set(row.display_order),
            endpoint_usage: Set(row.endpoint_usage),
            created_at: Set(row.created_at),
            updated_at: Set(row.updated_at),
            updated_by: Set(row.updated_by),
        })
        .collect::<Vec<_>>();
    insert_no_conflict::<captcha_config::Entity, _>(pg, models).await?;
    println!("captcha_configs:        {count}");
    Ok(())
}

async fn migrate_server_settings(
    mysql: &DatabaseConnection,
    pg: &DatabaseConnection,
) -> Result<()> {
    let rows = server_setting::Entity::find().all(mysql).await?;
    let count = rows.len();
    let models = rows
        .into_iter()
        .map(|row| server_setting::ActiveModel {
            id: Set(row.id),
            setting_key: Set(row.setting_key),
            value: Set(row.value),
            description: Set(row.description),
            created_at: Set(row.created_at),
            updated_at: Set(row.updated_at),
        })
        .collect::<Vec<_>>();
    insert_no_conflict::<server_setting::Entity, _>(pg, models).await?;
    println!("server_settings:        {count}");
    Ok(())
}

async fn migrate_daily_stats(mysql: &DatabaseConnection, pg: &DatabaseConnection) -> Result<()> {
    let rows = daily_stat::Entity::find().all(mysql).await?;
    let count = rows.len();
    let models = rows
        .into_iter()
        .map(|row| daily_stat::ActiveModel {
            date: Set(row.date),
            board_key: Set(row.board_key),
            total_responses: Set(row.total_responses),
            new_threads: Set(row.new_threads),
            ..Default::default()
        })
        .collect::<Vec<_>>();

    daily_stat::Entity::insert_many(models)
        .on_conflict(
            OnConflict::columns([daily_stat::Column::Date, daily_stat::Column::BoardKey])
                .update_columns([
                    daily_stat::Column::TotalResponses,
                    daily_stat::Column::NewThreads,
                ])
                .to_owned(),
        )
        .exec(pg)
        .await?;
    println!("daily_stats:            {count}");
    Ok(())
}
