use actix_web::{
    get, post,
    web::{Data, Json, Path},
    HttpResponse, Responder,
};
use serde_json::json;
use sqlx::{query, query_as};
use crate::schema::db;
mod schema;

#[post("/forms/mproj")]
pub async fn submit_mproj(state: Data<AppState>, body: MajorProjectSubmission) -> impl Responder {
    match query!("INSERT INTO major_projects(uid, name, description, active, status, date) VALUES ($1, $2, $3, $4, $5, $6)", body.uid, body.name, body.description, true, MajorProjectStatus::Pending, Utc::now().naive_utc())
        .execute(&state.db)
        .await
    {
        Ok(_) => HttpResponse::Ok(),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
    // TODO: slack ping
}

#[get("/form/mproj")]
pub async fn get_mprojs(state: Data<AppState>) -> impl Responder {
    match query_as!(MajorProject, "SELECT * FROM major_projects WHERE timestamp > $1", &state.year_start)
        .execute(&state.db)
        .await
    {
        Ok(mprojs) => HttpResponse::Ok().json(mprojs),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[put("/form/mproj/{id}")]
pub async fn edit_mproj(state: Data<AppState>, body: MajorProjectSubmission) -> impl Responder {
    // TODO: no editing if not pending
    let (id,) = path.into_inner();
    // TODO: fix date updating
    match query!("DELETE FROM major_projects WHERE id = $1; INSERT INTO major_projects(uid, name, description, active, status, date) VALUES ($2, $3, $4, $5, $6, $7)", id, body.uid, body.name, body.description, true, MajorProjectStatus::Pending, Utc::now().naive_utc())
        .execute(&state.db)
        .await
    {
        Ok(_) => HttpResponse::Ok(),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[put("/form/mproj-eboard/{id}")]
pub async fn edit_mproj(state: Data<AppState>, body: MajorProjectSubmissionEboard) -> impl Responder {
    let (id,) = path.into_inner();
    // TODO: fix date updating
    match query!("DELETE FROM major_projects WHERE id = $1; INSERT INTO major_projects(uid, name, description, active, status, date) VALUES ($2, $3, $4, $5, $6, $7)", id, body.uid, body.name, body.description, true, body.status, Utc::now().naive_utc())
        .execute(&state.db)
        .await
    {
        Ok(_) => HttpResponse::Ok(),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}