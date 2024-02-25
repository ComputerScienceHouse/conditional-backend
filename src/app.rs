use crate::{
    api::attendance::house::*,
    api::attendance::meeting::*,
    api::evals::routes::*,
    api::forms::coop::*,
    api::forms::intro_evals::*,
    api::forms::major_projects::*,
    api::users::routes::*,
    ldap::client::LdapClient,
    ldap::*,
    schema::{api, db},
};
use actix_web::web::{self, scope, Data};
use log::{log, Level};
use sqlx::{postgres::PgPoolOptions, query_as, Pool, Postgres};
use std::env;
use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify, OpenApi,
};
use utoipa_swagger_ui::SwaggerUi;

pub struct AppState {
    pub db: Pool<Postgres>,
    pub packet_db: Pool<Postgres>,
    pub eval_block_id: i32,
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
            get_user_intro_form,
            get_all_intro_forms,
            submit_intro_form,
            update_intro_form,
            submit_hm_attendance,
            count_hm_absences,
            get_hm_absences_by_user,
            get_hm_attendance_by_user_evals,
            modify_hm_attendance,
            get_voting_count,
            get_active_count,
            search_members,
            all_members,
            convert_freshman_user,
            get_intro_member_evals,
            get_member_evals,
            get_conditional,
            get_gatekeep,
            get_coop_form,
            get_coop_forms,
            submit_coop_form,
            get_user_major_projects,
            get_all_major_projects,
            submit_major_project,
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
                api::IntroForm,
                api::Absences,
                api::AbsenceWrapper,
                api::DateWrapper,
                IntroFormSubmission,
                api::FreshmanUpgrade,
                user::LdapUser,
                api::IntroStatus,
                api::MemberStatus,
                api::CoopSubmission,
                api::MajorProjectSubmission,
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
        scope("/api")
            .service(
                scope("/attendance")
                    // Directorship/Seminar routes
                    .service(submit_meeting_attendance)
                    .service(get_user_directorships)
                    .service(get_user_seminars)
                    .service(get_attendance_history)
                    .service(delete_meeting)
                    .service(modify_attendance)
                    .service(submit_hm_attendance)
                    .service(count_hm_absences)
                    .service(get_hm_absences_by_user)
                    .service(get_hm_attendance_by_user_evals)
                    .service(modify_hm_attendance),
            )
            .service(
                scope("/evals")
                    // Evals routes
                    .service(get_intro_member_evals)
                    .service(get_member_evals)
                    .service(get_conditional)
                    .service(get_gatekeep),
            )
            .service(
                scope("/forms")
                    // Intro forms
                    .service(get_user_intro_form)
                    .service(get_all_intro_forms)
                    .service(submit_intro_form)
                    .service(update_intro_form)
                    .service(get_coop_form)
                    .service(get_coop_forms)
                    .service(submit_coop_form)
                    .service(get_user_major_projects)
                    .service(get_all_major_projects)
                    .service(submit_major_project),
            )
            .service(
                scope("/users")
                    // User routes
                    .service(get_voting_count)
                    .service(get_active_count)
                    .service(search_members)
                    .service(all_members)
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
    log!(Level::Info, "Successfully opened conditional db connection");
    sqlx::migrate!()
        .run(&conditional_pool)
        .await
        .expect("Migration failed to run");

    let packet_pool = PgPoolOptions::new()
        .connect(&env::var("PACKET_DATABASE_URL").expect("PACKET_DATABASE_URL Not set"))
        .await
        .expect("Could not connect to database");
    log!(Level::Info, "Successfully opened packet db connection");

    let ldap = LdapClient::new(
        env::var("CONDITIONAL_LDAP_BIND_DN")
            .expect("CONDITIONAL_LDAP_BIND_DN not set")
            .as_str(),
        env::var("CONDITIONAL_LDAP_BIND_PW")
            .expect("CONDITIONAL_LDAP_BIND_PW not set")
            .as_str(),
    )
    .await;
    let evals_block_id = query_as!(
        db::ID,
        r#"
        SELECT current_eval_block as "id"
        FROM settings
        "#
    )
    .fetch_one(&conditional_pool)
    .await
    .expect("Could not retrieve settings.");
    Data::new(AppState {
        db: conditional_pool,
        packet_db: packet_pool,
        eval_block_id: *evals_block_id,
        ldap,
    })
}
