use sqlx::{Pool, Postgres};

pub struct AppState {
    pub db: Pool<Postgres>,
}

pub struct ID {
    pub id: i32,
}
