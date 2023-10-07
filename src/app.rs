use crate::api::attendance::{directorship::*, seminar::*, house::*};
use crate::schema::{
    api::{Directorship, MeetingAttendance, Seminar},
    db::CommitteeType,
};
use actix_web::web::{self, scope, Data};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::env;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub struct AppState {
    pub db: Pool<Postgres>,
    pub year_start: chrono::NaiveDateTime,
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

    let openapi = ApiDoc::openapi();

    cfg.service(
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
            .service(edit_directorship_attendance)
            // House meeting routes
            .service(submit_hm_attendance)
    )
    .service(SwaggerUi::new("/docs/{_:.*}").url("/api-doc/openapi.json", openapi));
}

pub async fn get_app_data() -> Data<AppState> {
    let pool = PgPoolOptions::new()
        .connect(&env::var("DATABASE_URL").expect("DATABASE_URL Not set"))
        .await
        .expect("Could not connect to database");
    println!("Successfully opened db connection");
    Data::new(AppState {
        db: pool,
        year_start: NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2023, 6, 1).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        ),
    })
}
