use crate::{
    api::{log_query_as, log_query, open_transaction},
    app::AppState,
    schema::api::*,
};
use actix_web::{
    get, post,
    web::{Data, Path},
    HttpResponse, Responder,
};
use sqlx::{query, query_as};
use log::{log, Level};


#[get("/queue")]
pub async fn get_housing_queue(state: Data<AppState>) -> impl Responder {
    match log_query_as(query_as!(UID, "select uid from in_housing_queue").fetch_all(&state.db).await, None).await {
        Ok((_, i)) => HttpResponse::Ok().json(i),
        Err(res) => return res,
    }
}

#[post("/queue/{user}")]
pub async fn add_member_to_housing_queue(path: Path<(String,)>, state: Data<AppState>) -> impl Responder {
    let user = path.into_inner().0;

    let mut transaction = match open_transaction(&state.db).await {
        Ok(t) => t,
        Err(res) => return res,
    };
    log!(Level::Trace, "Acquired transaction");

    match log_query(query!("INSERT INTO in_housing_queue (uid) VALUES ($1)", user).execute(&state.db).await.map(|_| ()), Some(transaction)).await {
        Ok(tx) => transaction = tx.unwrap(),
        Err(res) => return res,
    }

    match transaction.commit().await {
        Ok(_) => HttpResponse::Created().finish(),
        Err(e) => {
            log!(Level::Error, "Transaction failed to commit");
            HttpResponse::InternalServerError().body(e.to_string())
        }
    }
}
