use crate::auth::CSHAuth;
use crate::ldap;
use crate::schema::api::FreshmanUpgrade;
use crate::{app::AppState, schema::api::NewIntroMember};
use actix_web::{
    get, post, put,
    web::{Data, Json, Path},
    HttpResponse, Responder,
};
use log::{log, Level};

#[utoipa::path(
    context_path="/api/users",
    responses(
        (status = 200, description = "The number of active voting members"),
        )
    )]
#[get("/voting_count", wrap = "CSHAuth::enabled()")]
pub async fn get_voting_count(state: Data<AppState>) -> impl Responder {
    log!(Level::Info, "GET /users/voting_count");
    HttpResponse::Ok().body(format!(
        "{}",
        ldap::get_active_upperclassmen(&state.ldap).await.len()
    ))
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
    HttpResponse::Ok().body(format!(
        "{}",
        ldap::get_group_members(&state.ldap, "active").await.len()
    ))
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
    HttpResponse::Ok().json(ldap::search_users(&state.ldap, query.as_str()).await)
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
    HttpResponse::Ok().json(ldap::get_group_members(&state.ldap, "member").await)
}

#[utoipa::path(
    context_path="/api/users",
    responses(
        (status = 200, description = "Freshman user successfully created"),
        )
    )]
#[post("/", wrap = "CSHAuth::evals_only()")]
pub async fn create_freshman_user(
    state: Data<AppState>,
    body: Json<NewIntroMember>,
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
    state: Data<AppState>,
    body: Json<FreshmanUpgrade>,
    path: Path<(String,)>,
) -> impl Responder {
    let user = path.into_inner().0;
    log!(Level::Info, "PUT /users/{user}");
    HttpResponse::NotImplemented().finish()
}
