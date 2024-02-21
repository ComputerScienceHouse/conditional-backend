use actix_web::{
    error, get,
    http::{header::ContentType, StatusCode},
    HttpResponse,
};
use derive_more::{Display, Error};
use log::{log, Level};
use sqlx::{query_as, Pool, Postgres, Transaction};

/// Error wrapper around sqlx::Error and actix_web::error::ResponseError
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
