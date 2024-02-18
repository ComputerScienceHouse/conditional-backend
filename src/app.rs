use crate::{
    api::attendance::meeting::*,
    ldap::{client::LdapClient, user::LdapUser},
    schema::{api, db},
};
use actix_web::web::{self, scope, Data};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use futures::lock::Mutex;
use openssl::pkey::{PKey, Public};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::{collections::HashMap, env, sync::Arc};
use utoipa::{
    openapi::security::{ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify, OpenApi,
};
use utoipa_swagger_ui::SwaggerUi;

pub struct AppState {
    pub db: Pool<Postgres>,
    pub packet_db: Pool<Postgres>,
    pub year_start: chrono::NaiveDateTime,
    pub ldap: LdapClient,
}

pub fn configure_app(cfg: &mut web::ServiceConfig) {
    #[derive(OpenApi)]
    #[openapi(
        paths(
            submit_meeting_attendance,
            get_user_directorships,
            get_user_seminars,
            get_attendance_history,
            delete_meeting,
            modify_attendance,
        ),
        components(
            schemas(
                MeetingSubmission,
                ModifyMeetingParameters,
                DeleteMeetingParameters,
                db::MeetingType,
                api::Meeting,
                api::MeetingAttendance,
                api::User,
                
            )
        ),
        modifiers(&SecurityAddon),
        tags(
            (name = "Conditional", description = "Conditional Actix API")
        ),
    )]
    struct ApiDoc;

    struct SecurityAddon;

    impl Modify for SecurityAddon {
        fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
            let components = openapi.components.as_mut().unwrap();
            components.add_security_scheme(
                "csh",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
            components.add_security_scheme(
                "intro",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }

    let openapi = ApiDoc::openapi();

    cfg.service(
        scope("/api").service(
            scope("/attendance")
                // Directorship/Seminar routes
                .service(submit_meeting_attendance)
                .service(get_user_directorships)
                .service(get_user_seminars)
                .service(get_attendance_history)
                .service(delete_meeting)
                .service(modify_attendance),
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
    sqlx::migrate!()
        .run(&conditional_pool)
        .await
        .expect("Migration failed to run");
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
    })
}
