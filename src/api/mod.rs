use actix_web::HttpResponse;
use log::{log, Level};
use sqlx::{Error, Pool, Postgres, Transaction};

pub mod attendance {
    pub mod routes;
}

pub mod forms {
    pub mod routes;
}

pub async fn open_transaction(db: &Pool<Postgres>) -> Result<Transaction<Postgres>, HttpResponse> {
    match db.try_begin().await {
        Ok(Some(t)) => Ok(t),
        Ok(None) => {
            log!(Level::Error, "Failed to open transaction");
            Err(HttpResponse::InternalServerError().body("Internal DB Error: Ok(None) transaction"))
        }
        Err(e) => {
            log!(Level::Error, "Failed to open transaction");
            Err(HttpResponse::InternalServerError().body(format!("Internal DB Error: {}", e)))
        }
    }
}

pub async fn log_query_as<T>(
    query: Result<Vec<T>, Error>,
    tx: Transaction<'_, Postgres>,
) -> Result<(Transaction<'_, Postgres>, Vec<T>), HttpResponse> {
    match query {
        Ok(v) => Ok((tx, v)),
        Err(e) => {
            log!(Level::Warn, "DB Query failed: {}", e);
            match tx.rollback().await {
                Ok(_) => {}
                Err(tx_e) => {
                    log!(Level::Error, "Transaction failed to rollback: {}", tx_e);
                    return Err(HttpResponse::InternalServerError().body("Internal DB Error"));
                }
            }
            return Err(HttpResponse::InternalServerError().body("Internal DB Error"));
        }
    }
}

pub async fn log_query(
    query: Result<(), Error>,
    tx: Transaction<'_, Postgres>,
) -> Result<Transaction<'_, Postgres>, HttpResponse> {
    match query {
        Ok(_) => Ok(tx),
        Err(e) => {
            log!(Level::Warn, "DB Query failed: {}", e);
            match tx.rollback().await {
                Ok(_) => {}
                Err(tx_e) => {
                    log!(Level::Error, "Transaction failed to rollback: {}", tx_e);
                    return Err(HttpResponse::InternalServerError().body("Internal DB Error"));
                }
            }
            return Err(HttpResponse::InternalServerError().body("Internal DB Error"));
        }
    }
}
