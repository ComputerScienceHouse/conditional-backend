use crate::{
    api::{
        attendance::{directorship::*, seminar::*},
        evals::routes::*,
        users::routes::*,
    },
    ldap::{client::LdapClient, user::LdapUser},
    schema::{
        api::{Directorship, FreshmanUpgrade, IntroStatus, MemberStatus, NewIntroMember, Seminar},
        db::CommitteeType,
    },
};
use actix_web::web::{self, scope, Data};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use futures::lock::Mutex;
use openssl::pkey::{PKey, Public};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::{collections::HashMap, env, sync::Arc};
use utoipa::{
    openapi::security::{ApiKey, ApiKeyValue, SecurityScheme},
    Modify, OpenApi,
};
use utoipa_swagger_ui::SwaggerUi;

pub struct AppState {
    pub db: Pool<Postgres>,
    pub packet_db: Pool<Postgres>,
    pub year_start: chrono::NaiveDateTime,
    pub ldap: LdapClient,
    pub jwt_cache: Arc<Mutex<HashMap<String, PKey<Public>>>>,
}

pub fn configure_app(cfg: &mut web::ServiceConfig) {
    #[derive(OpenApi)]
    #[openapi(
        paths(
            // attendance/seminar
            submit_seminar_attendance,
            get_seminars_by_user,
            get_seminars,
            delete_seminar,
            edit_seminar_attendance,
            // attendance/directorship
            submit_directorship_attendance,
            get_directorships_by_user,
            get_directorships,
            edit_directorship_attendance,
            delete_directorship,
            // evals
            get_intro_evals_wrapper,
            get_member_evals,
            get_gatekeep,
            // user
            get_voting_count,
            get_active_count,
            search_members,
            all_members,
            create_freshman_user,
            convert_freshman_user,
        ),
        components(schemas(Seminar, Directorship, CommitteeType, LdapUser, NewIntroMember, FreshmanUpgrade, MemberStatus, IntroStatus)),

        tags(
            (name = "Conditional", description = "Conditional Actix API")
            ),
        modifiers(&SecurityAddon)
    )]
    struct ApiDoc;

    struct SecurityAddon;

    impl Modify for SecurityAddon {
        fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
            let components = openapi.components.as_mut().unwrap();
            components.add_security_scheme(
                "api_key",
                SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("frontend_api_key"))),
            )
        }
    }

    let openapi = ApiDoc::openapi();

    cfg.service(
        scope("/api")
            .service(
                scope("/attendance")
                    // Seminar routes
                    .service(submit_seminar_attendance)
                    .service(get_seminars_by_user)
                    .service(get_seminars)
                    .service(delete_seminar)
                    .service(edit_seminar_attendance)
                    // Directorship routes
                    .service(submit_directorship_attendance)
                    .service(get_directorships_by_user)
                    .service(get_directorships)
                    .service(delete_directorship)
                    .service(edit_directorship_attendance),
            )
            .service(
                scope("/evals")
                    // Evals routes
                    .service(get_intro_evals_wrapper)
                    .service(get_member_evals)
                    .service(get_conditional)
                    .service(get_gatekeep),
            )
            .service(
                scope("/users")
                    // User routes
                    .service(get_voting_count)
                    .service(get_active_count)
                    .service(search_members)
                    .service(all_members)
                    .service(create_freshman_user)
                    .service(convert_freshman_user),
            ),
    )
    .service(SwaggerUi::new("/docs/{_:.*}").url("/api-doc/openapi.json", openapi));
}

pub async fn get_app_data() -> Data<AppState> {
    let conditional_pool = PgPoolOptions::new()
        .connect(&env::var("DATABASE_URL").expect("DATABASE_URL Not set"))
        .await
        .expect("Could not connect to database");
    println!("Successfully opened conditional db connection");
    let packet_pool = PgPoolOptions::new()
        .connect(&env::var("PACKET_DATABASE_URL").expect("PACKET_DATABASE_URL Not set"))
        .await
        .expect("Could not connect to database");
    println!("Successfully opened packet db connection");
    let ldap = LdapClient::new(
        &env::var("CONDITIONAL_LDAP_BIND_DN")
            .expect("CONDITIONAL_LDAP_BIND_DN not set")
            .as_str(),
        &env::var("CONDITIONAL_LDAP_BIND_PW")
            .expect("CONDITIONAL_LDAP_BIND_PW not set")
            .as_str(),
    )
    .await;
    Data::new(AppState {
        db: conditional_pool,
        packet_db: packet_pool,
        year_start: NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2023, 6, 1).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        ),
        ldap,
        jwt_cache: Arc::new(Mutex::new(HashMap::new())),
    })
}
