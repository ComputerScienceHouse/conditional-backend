use actix_web::web::{self, scope, Data};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::env;

use crate::api::attendance::{directorship::*, routes::*};

pub struct AppState {
    pub db: Pool<Postgres>,
    pub year_start: chrono::NaiveDateTime,
}

pub fn configure_app(cfg: &mut web::ServiceConfig) {
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
            .service(edit_directorship_attendance),
    );
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
