use crate::api::lib::{open_transaction, UserError};
use crate::app::AppState;
use crate::auth::CSHAuth;
use crate::ldap;
use crate::schema::api::{FreshmanUpgrade, User};
use actix_web::{
    get, put,
    web::{Data, Json, Path},
    HttpResponse, Responder,
};
use sqlx::{query, query_as};

#[utoipa::path(
    context_path = "/api/users",
    tag = "Users",
    responses(
        (status = 200, description = "The number of active upperclassmen", body = i32),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    ),
    security(
        ("csh" = []),
        ("intro" = []),
    )
)]
#[get("/voting_count", wrap = "CSHAuth::member_and_intro()")]
pub async fn get_voting_count(state: Data<AppState>) -> Result<impl Responder, UserError> {
    Ok(HttpResponse::Ok().json(
        ldap::get_active_upperclassmen(&state.ldap)
            .await
            .map_err(|_| UserError::ServerError)?
            .len(),
    ))
}

#[utoipa::path(
    context_path="/api/users",
    tag = "Users",
    responses(
        (status = 200, description = "The number of active members", body = i32),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    ),
    security(
        ("csh" = []),
        ("intro" = []),
    )
)]
#[get("/active_count", wrap = "CSHAuth::member_and_intro()")]
pub async fn get_active_count(state: Data<AppState>) -> Result<impl Responder, UserError> {
    Ok(HttpResponse::Ok().json(
        ldap::get_group_members(&state.ldap, "active")
            .await
            .map_err(|_| UserError::ServerError)?
            .len(),
    ))
}

#[utoipa::path(
    context_path="/api/users",
    tag = "Users",
    responses(
        (status = 200, description = "A list of members matching the search string", body = Vec<LdapUser>),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    ),
    params(
        ("query" = String, Path, description = "Query")
    ),
    security(
        ("csh" = []),
        ("intro" = []),
    )
)]
#[get("/search/{query}", wrap = "CSHAuth::member_and_intro()")]
pub async fn search_members(
    state: Data<AppState>,
    path: Path<(String,)>,
) -> Result<impl Responder, UserError> {
    Ok(HttpResponse::Ok().json(
        ldap::search_users(&state.ldap, path.0.as_str())
            .await
            .map_err(|_| UserError::ServerError)?,
    ))
}

#[utoipa::path(
    context_path="/api/users",
    tag = "Users",
    responses(
        (status = 200, description = "Gets All members", body = Vec<User>),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    ),
    security(
        ("csh" = []),
        ("intro" = []),
    )
)]
#[get("/all", wrap = "CSHAuth::member_and_intro()")]
pub async fn all_members(state: Data<AppState>) -> Result<impl Responder, UserError> {
    Ok(HttpResponse::Ok().json(
        query_as!(
            User,
            r#"select id uid, name, rit_username, csh_username, is_csh, is_intro from "user""#
        )
        .fetch_all(&state.db)
        .await?,
    ))
}

#[utoipa::path(
    context_path="/api/users",
    tag = "Users",
    responses(
        (status = 200, description = "Freshman user successfully converted to member"),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    ),
    security(
        ("csh" = ["eboard"]),
    )
)]
#[put("/", wrap = "CSHAuth::evals_only()")]
pub async fn convert_freshman_user(
    state: Data<AppState>,
    body: Json<FreshmanUpgrade>,
) -> Result<impl Responder, UserError> {
    let mut transaction = open_transaction(&state.db).await?;
    query!(
        r#"update "user" set is_csh = true, is_intro = false, ipa_unique_id = $1"#,
        body.ipa_unique_id
    )
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(HttpResponse::Ok().finish())
}
