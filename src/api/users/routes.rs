use crate::api::log_query_as;
use crate::app::AppState;
use crate::ldap::{get_active_upperclassmen, get_intro_members, get_user};
use crate::schema::api::{IntroStatus, MemberStatus, Packet};
use actix_web::{
    get,
    web::{Data, Path},
    HttpResponse, Responder,
};
use log::{log, Level};
use sqlx::{query, query_as, Pool, Postgres, Transaction};
use utoipa::openapi::security::Http;

#[utoipa::path(
    context_path="/users",
    responses(
        (status = 200, description = "Gets a list of active members", body = [MemberStatus]),
        (status = 500, description = "Error created by Query"),
        )
    )]
#[get("/voting_count", wrap = "CSHAuth::enabled()")]
pub async fn get_voting_count(state: Data<AppState>) -> impl Responder {
    log!(Level::Info, "GET /users/voting_count");
    match ldap::get_active_upperclassmen(&state.ldap).await {
        Ok(v) => HttpResponse::Ok().body(format!("{}", v.len())),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[utoipa::path(
    context_path="/api/users",
    responses(
        (status = 200, description = "The number of active members"),
        )
    )]
#[get("/active_count", wrap = "CSHAuth::enabled()")]
pub async fn get_active_count(state: Data<AppState>) -> impl Responder {
    log!(Level::Info, "GET /users/active_count");
    match ldap::get_group_members(&state.ldap, "active").await {
        Ok(v) => HttpResponse::Ok().body(format!("{}", v.len())),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[utoipa::path(
    context_path="/api/users",
    responses(
        (status = 200, description = "A list of members matching the search string", body = [LdapUser]),
        )
    )]
#[get("/search/{query}", wrap = "CSHAuth::enabled()")]
pub async fn search_members(state: Data<AppState>, path: Path<(String,)>) -> impl Responder {
    let query = path.into_inner().0;
    log!(Level::Info, "GET /users/search/{}", query);
    match ldap::search_users(&state.ldap, query.as_str()).await {
        Ok(v) => HttpResponse::Ok().json(v),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[utoipa::path(
    context_path="/api/users",
    responses(
        (status = 200, description = "Gets All members", body = [LdapUser]),
        )
    )]
#[get("/all", wrap = "CSHAuth::enabled()")]
pub async fn all_members(state: Data<AppState>) -> impl Responder {
    log!(Level::Info, "GET /users/all");
    match ldap::get_group_members(&state.ldap, "member").await {
        Ok(v) => HttpResponse::Ok().json(v),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[utoipa::path(
    context_path="/api/users",
    responses(
        (status = 200, description = "Freshman user successfully created"),
        )
    )]
#[post("/", wrap = "CSHAuth::evals_only()")]
pub async fn create_freshman_user(
    _state: Data<AppState>,
    _body: Json<NewIntroMember>,
) -> impl Responder {
    log!(Level::Info, "POST /users");
    HttpResponse::NotImplemented().finish()
}

#[utoipa::path(
    context_path="/api/users",
    responses(
        (status = 200, description = "Freshman user successfully converted to member"),
        )
    )]
#[put("/{user}", wrap = "CSHAuth::evals_only()")]
pub async fn convert_freshman_user(
    _state: Data<AppState>,
    _body: Json<FreshmanUpgrade>,
    path: Path<(String,)>,
) -> impl Responder {
    let user = path.into_inner().0;
    log!(Level::Info, "PUT /users/{user}");
    HttpResponse::NotImplemented().finish()
}
