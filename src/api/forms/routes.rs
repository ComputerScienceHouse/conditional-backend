// use crate::schema::db;
// use actix_web::{
// get, post,
// web::{Data, Json, Path},
// HttpResponse, Responder,
// };
// use serde_json::json;
// use sqlx::{query, query_as};
//
// #[post("/forms/mproj")]
// pub async fn submit_mproj(state: Data<AppState>, body:
// MajorProjectSubmission) -> impl Responder { match query!("INSERT INTO
// major_projects(uid, name, description, active, status, date) VALUES ($1, $2,
// $3, $4, $5, $6)", body.uid, body.name, body.description, true,
// MajorProjectStatus::Pending, Utc::now().naive_utc()) .execute(&state.db)
// .await
// {
// Ok(_) => HttpResponse::Ok(),
// Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
// }
// TODO: slack ping
// }
//
// #[get("/forms/mproj")]
// pub async fn get_mprojs(state: Data<AppState>) -> impl Responder {
// match query_as!(
// MajorProject,
// "SELECT * FROM major_projects WHERE timestamp > $1",
// &state.year_start
// )
// .execute(&state.db)
// .await
// {
// Ok(mprojs) => HttpResponse::Ok().json(mprojs),
// Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
// }
// }
//
// #[put("/forms/mproj/{id}")]
// pub async fn edit_mproj(state: Data<AppState>, body: MajorProjectSubmission)
// -> impl Responder { TODO: no editing if not pending
// let (id,) = path.into_inner();
// TODO: fix date updating (use UPDATE i am bozo)
// match query!("DELETE FROM major_projects WHERE id = $1; INSERT INTO
// major_projects(uid, name, description, active, status, date) VALUES ($2, $3,
// $4, $5, $6, $7)", id, body.uid, body.name, body.description, true,
// MajorProjectStatus::Pending, Utc::now().naive_utc()) .execute(&state.db)
// .await
// {
// Ok(_) => HttpResponse::Ok(),
// Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
// }
// }
//
// #[put("/forms/mproj-eboard/{id}")]
// pub async fn edit_mproj(
// state: Data<AppState>,
// body: MajorProjectSubmissionEboard,
// ) -> impl Responder {
// let (id,) = path.into_inner();
// TODO: fix date updating (use UPDATE i am bozo)
// match query!("DELETE FROM major_projects WHERE id = $1; INSERT INTO
// major_projects(uid, name, description, active, status, date) VALUES ($2, $3,
// $4, $5, $6, $7)", id, body.uid, body.name, body.description, true,
// body.status, Utc::now().naive_utc()) .execute(&state.db)
// .await
// {
// Ok(_) => HttpResponse::Ok(),
// Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
// }
// }
//
// #[post("/forms/coop")]
// pub async fn submit_coop(state: Data<AppState>, body: CoopSubmission) -> impl
// Responder { match query!(
// "INSERT INTO current_coops(uid, date_created, semester) VALUES ($1, $2, $3)",
// body.uid,
// body.date,
// body.semester
// )
// .execute(&state.db)
// .await
// {
// Ok(_) => HttpResponse::Ok(),
// Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
// }
// }
//
// #[get("/forms/coop")]
// pub async fn get_coops(state: Data<AppState>) -> impl Responder {
// match query_as!(
// Coop,
// "SELECT * FROM major_project WHERE timestamp > $1",
// state.year_start
// )
// .execute(&state.db)
// .await
// {
// Ok(coops) => HttpResponse::Ok().json(coops),
// Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
// }
// }
//
// #[post("/forms/intro")]
// #[put("/forms/intro")]
// pub async fn submit_intro_form(state: Data<AppState>, body:
// IntroFormSubmission) -> impl Responder { match query!(
// "UPDATE freshman_eval_data SET social_events = $1, other_notes = $2 WHERE uid
// = $3", body.social_events,
// body.comments,
// body.uid
// )
// .execute(&state.db)
// .await
// {
// Ok(_) => HttpResponse::Ok(),
// Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
// }
// }
//
// #[get("/forms/intro/{user}")]
// pub async fn get_intro_form(state: Data<AppState>) -> impl Responder {
// let (user,) = path.into_inner();
// match query_as!(
// FreshmanEvaluation,
// "SELECT * FROM freshman_eval_data WHERE eval_date > $1 uid = $2",
// user,
// &state.year_start
// )
// .fetch_all(&state.db)
// .await
// {
// Ok(form) => HttpResponse::Ok().json(form),
// Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
// }
// }
//
// #[get("/forms/intro")]
// pub async fn get_intro_form(state: Data<AppState>) -> impl Responder {
// match query_as!(
// FreshmanEvaluation,
// "SELECT * FROM freshman_eval_data WHERE eval_date > $1",
// &state.year_start
// )
// .fetch_all(&state.db)
// .await
// {
// Ok(forms) => HttpResponse::Ok().json(forms),
// Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
// }
// }
//
