use actix_web::web::{self, scope, Data};
use conditional_backend::models::AppState;
use sqlx::postgres::PgPoolOptions;
use std::env;
// use utoipa::OpenApi;

pub async fn get_app_data() -> Data<AppState> {
    let pool = PgPoolOptions::new()
        .connect(&env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();
    Data::new(AppState { db: pool })
}
