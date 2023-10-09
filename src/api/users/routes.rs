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
#[get("/active")]
pub async fn get_active_members(state: Data<AppState>) -> impl Responder {
