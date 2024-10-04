use crate::{
    api::{
        attendance::{house::*, meeting::*},
        batch::routes::*,
        evals::routes::*,
        forms::{coop::*, intro_evals::*, major_project::*},
        housing::routes::*,
        users::routes::*,
    },
    ldap::{self, client::LdapClient},
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
            get_intro_form,
            get_all_intro_forms,
            submit_intro_form,
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
            get_housing_queue,
            add_to_housing_queue,
            remove_from_housing_queue,
            get_rooms,
            add_user_to_room,
            remove_user_from_room,
            get_freshman_room_number,
            get_member_room_number,
            get_all_batches,
            create_batch,
            pull_user,
            submit_batch_pr,
            get_pull_requests,
            pass_batch,
            fail_batch
        ),
        components(
            schemas(
                MeetingSubmission,
                ModifyMeetingParameters,
                DeleteMeetingParameters,
                db::MeetingType,
                api::Meeting,
                api::MeetingAttendance,
                api::HouseAttendance,
                api::HouseAttendanceUpdate,
                api::User,
                api::IntroForm,
                api::Absences,
                api::AbsenceWrapper,
                api::DateWrapper,
                api::IntroForm,
                api::FreshmanUpgrade,
                ldap::user::LdapUser,
                api::IntroStatus,
                api::MemberStatus,
                api::GatekeepStatus,
                api::CoopSubmission,
                api::MajorProjectSubmission,
                api::Room,
                api::Batch,
                api::BatchSubmission,
                api::BatchPull,
                api::BatchConditionSubmission,
                db::SemesterEnum,
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
                scope("/batch")
                    // Batch routes
                    .service(get_all_batches)
                    .service(create_batch)
                    .service(pull_user)
                    .service(submit_batch_pr)
                    .service(get_pull_requests)
                    .service(pass_batch)
                    .service(fail_batch),
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
                    .service(get_intro_form)
                    .service(get_all_intro_forms)
                    .service(submit_intro_form)
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
            )
            .service(
                scope("/housing")
                    // Housing routes
                    .service(get_housing_queue)
                    .service(add_to_housing_queue)
                    .service(remove_from_housing_queue)
                    .service(get_rooms)
                    .service(add_user_to_room)
                    .service(remove_user_from_room)
                    .service(get_freshman_room_number)
                    .service(get_member_room_number),
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
