use actix_web::web::{self, scope, Data};
use sqlx::postgres::PgPoolOptions;
use std::env;
// use utoipa::OpenApi;

use sqlx::{Pool, Postgres};

pub struct AppState {
    pub db: Pool<Postgres>,
    pub year_start: chrono::NaiveDateTime,
}

pub async fn get_app_data() -> Data<AppState> {
    let pool = PgPoolOptions::new()
        .connect(&env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();
    Data::new(AppState {
        db: pool,
        year_start: chrono::NaiveDateTime::MAX,
    })
}
