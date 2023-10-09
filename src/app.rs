use crate::{
    api::attendance::{directorship::*, seminar::*},
    ldap::client::LdapClient,
    schema::{
        api::{Directorship, MeetingAttendance, Seminar},
        db::CommitteeType,
    },
};
use actix_web::web::{self, scope, Data};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use openssl::pkey::{PKey, Public};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::{
    collections::HashMap,
    env,
    sync::{Arc, Mutex},
};
use utoipa::{
    openapi::security::{ApiKey, ApiKeyValue, SecurityScheme},
    Modify, OpenApi,
};
use utoipa_swagger_ui::SwaggerUi;

pub struct AppState {
    pub db: Pool<Postgres>,
    pub year_start: chrono::NaiveDateTime,
    pub ldap: LdapClient,
    pub jwt_cache: Arc<Mutex<HashMap<String, PKey<Public>>>>,
}

pub fn configure_app(cfg: &mut web::ServiceConfig) {
    #[derive(OpenApi)]
    #[openapi(
        paths(
            submit_seminar_attendance,
            get_seminars_by_user,
            get_seminars,
            delete_seminar,
            edit_seminar_attendance,
            submit_directorship_attendance,
            get_directorships_by_user,
            get_directorships,
            edit_directorship_attendance,
            delete_directorship,
        ),
        components(schemas(Seminar, Directorship, MeetingAttendance, CommitteeType)),
        tags(
            (name = "Conditional", description = "Conditional Actix API")
            )
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
            .service(SwaggerUi::new("/docs/{_:.*}").url("/api-doc/openapi.json", openapi)),
    );
}

pub async fn get_app_data() -> Data<AppState> {
    let db = PgPoolOptions::new()
        .connect(&env::var("DATABASE_URL").expect("DATABASE_URL Not set"))
        .await
        .expect("Could not connect to database");
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
        db,
        year_start: NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2023, 6, 1).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        ),
        ldap,
    })
}
