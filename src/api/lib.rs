use actix_web::{
    error, get,
    http::{header::ContentType, StatusCode},
    App, HttpResponse, HttpServer,
};
use derive_more::{Display, Error};
use log::{log, Level};
use sqlx::{Error, Pool, Postgres, Transaction};

#[derive(Debug, Display, Error)]
pub enum UserError {
    #[display(fmt = "Invalid input: {} for field {}", value, field)]
    ValueError { value: String, field: String },
    #[display(fmt = "An internal server error occurred. Please contact an RTP.")]
    ServerError,
    #[display(fmt = "An internal database error occurred. Please contact an RTP.")]
    DatabaseError,
}

impl error::ResponseError for UserError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::html())
            .body(self.to_string())
    }
    fn status_code(&self) -> StatusCode {
        match *self {
            UserError::ValueError { .. } => StatusCode::BAD_REQUEST,
            UserError::ServerError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            UserError::DatabaseError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<sqlx::Error> for UserError {
    fn from(value: sqlx::Error) -> Self {
        log!(Level::Error, "{}", value.to_string());
        UserError::DatabaseError
    }
}

pub async fn open_transaction(db: &Pool<Postgres>) -> Result<Transaction<Postgres>, UserError> {
    match db.try_begin().await {
        Ok(Some(t)) => {
            log!(Level::Trace, "Acquired transaction");
            Ok(t)
        }
        _ => {
            log!(Level::Error, "Failed to open transaction");
            Err(UserError::DatabaseError)
        }
    }
}

pub async fn log_query_as<T>(
    query: Result<Vec<T>, Error>,
    tx: Option<Transaction<'_, Postgres>>,
) -> Result<(Option<Transaction<'_, Postgres>>, Vec<T>), HttpResponse> {
    match query {
        Ok(v) => Ok((tx, v)),
        Err(e) => {
            log!(Level::Warn, "DB Query failed: {}", e);
            if let Some(tx) = tx {
                match tx.rollback().await {
                    Ok(_) => {}
                    Err(tx_e) => {
                        log!(Level::Error, "Transaction failed to rollback: {}", tx_e);
                        return Err(HttpResponse::InternalServerError().body("Internal DB Error"));
                    }
                }
            }
            return Err(HttpResponse::InternalServerError().body("Internal DB Error"));
        }
    }
}

pub async fn log_query(
    query: Result<(), Error>,
    tx: Option<Transaction<'_, Postgres>>,
) -> Result<Option<Transaction<'_, Postgres>>, HttpResponse> {
    match query {
        Ok(_) => Ok(tx),
        Err(e) => {
            log!(Level::Warn, "DB Query failed: {}", e);
            if let Some(tx) = tx {
                match tx.rollback().await {
                    Ok(_) => {}
                    Err(tx_e) => {
                        log!(Level::Error, "Transaction failed to rollback: {}", tx_e);
                        return Err(HttpResponse::InternalServerError().body("Internal DB Error"));
                    }
                }
            }
            return Err(HttpResponse::InternalServerError().body("Internal DB Error"));
        }
    }
}
