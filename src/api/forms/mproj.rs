use crate::api::{log_query, log_query_as, open_transaction};
use crate::app::AppState;
use crate::schema::api::*;
use crate::schema::db::CommitteeType;

use actix_web::{
    delete, get, post, put,
    web::{Data, Json, Path},
    HttpResponse, Responder,
};
use log::{log, Level};
use sqlx::{query, query_as, Pool, Postgres, Transaction};

#[utoipa::path(
    context_path="/forms",
    responses(
        (status = 200, description = "Submits a new major project"),
        (status = 500, description = "Error created by Query"),
    )
)]
#[post("/mproj")]
pub async fn submit_mproj(
    state: Data<AppState>,
    body: Json<MajorProjectSubmission>,
) -> impl Responder {
    log!(Level::Info, "POST /forms/mproj");

    match query!(
        "INSERT INTO major_projects (uid, name, description, active, status, date) VALUES ($1, $2, $3, $4, $5, $6)",
        body.uid,
        body.name,
        body.description,
        true,
        MajorProjectStatus.PENDING,
        Utc::now().naive_utc()
    )
    .execute(&state.db)
    .await
    {
        Ok(tx) => transaction = tx.unwrap(),
        Err(e) => return e,
    }
    log!(Level::Trace, "Added major project");
    // TODO: slack ping
}

#[utoipa(
    context_path = "/forms",
    responses(
        (status = 200, description = "Get all current major projects"),
        (status = 500, description = "Error created by Query")
    )
)]
#[get("/mproj")]
pub async fn get_mprojs(
    state: Data<AppState>,
) -> impl Responder {
    log!(Level::Info, "GET /forms/mproj");

    match query_as!(
        MajorProject,
        "SELECT * FROM major_projects WHERE date > $1",
        &state.year_start
    )
    .fetch_all(&state.db)
    .await
    {
        Ok(mprojs) => HttpResponse::Ok().json(mprojs),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[utoipa(
    context_path = "/forms",
    responses(
        (status = 200, description = "Update specified major project (all members)"),
        (status = 500, description = "Error created by Query")
    )
)]
#[put("/mproj/{id}")]
pub async fn edit_mproj(
    path: Path<String,>,
    state: Data<AppState>,
    body: Json<MajorProjectSubmission>,
) -> impl Responder {
    let (id,) = path.into_inner();
    log!(Level::Info, "PUT /forms/mproj/{id}");

    let id = match id.parse::<i32>() {
        Ok(id) => id,
        Err(_) => {
            log!(Level::Warn, "Invalid id");
            return HttpResponse::BadRequest().body("Invalid id");
        }
    };

    match query!(
        "UPDATE major_projects SET name = $1, description = $2 WHERE id = $3",
        body.name,
        body.description,
        body.uid,
    )
    .execute(&state.db)
    .await
    {
        Ok(_) => HttpResponse::Ok(),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[utoipa(
    context_path = "/forms"
    responses(
        (status = 200, description = "PUT /forms/mproj-eboard/{id}")
        (status = 500, decription = "Error created by Query")
    )
)]
#[put("/mproj-eboard/{id}")]
pub async fn edit_mproj_eboard(
    path: Path<String,>,
    state: Data<AppState>,
    body: Json<MajorProjectSubmission>,
) -> impl Responder {
    let (id,) = path.into_inner();
    log!(Level::Info, "PUT /forms/mproj-eboard/{id}");

    let id = match id.parse::<i32>() {
        Ok(id) => id,
        Err(_) => {
            log!(Level::Warn, "Invalid id");
            return HttpResponse::BadRequest.body("Invalid id");
        }
    };

    match query!(
        "UPDATE major_projects SET status = $1 WHERE id = $2",
        body.status,
        body.uid,
    )
    .execute(&state.db)
    .await
    {
        Ok(_) => HttpResponse::Ok(),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}
