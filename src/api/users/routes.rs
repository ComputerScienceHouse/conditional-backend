use crate::api::lib::UserError;
use crate::app::AppState;
use crate::auth_service::CSHAuth;
use crate::schema::api::{FreshmanUpgrade, User};
use actix_web::{
    get, put,
    web::{Data, Json, Path},
    HttpResponse, Responder,
};
use sqlx::{query, query_as, Connection};

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
        state
            .ldap
            .get_active_upperclassmen()
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
        state
            .ldap
            .get_group_members("active")
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
        state
            .ldap
            .search_users(path.0.as_str())
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
            r#"select id uid, name, rit_username, csh_username from "user""#
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
    let mut conn = state.db.acquire().await?;
    conn.transaction(|txn| {
        Box::pin(async move {
            query!(
                r#"update "user" set ipa_unique_id = $1"#,
                body.ipa_unique_id
            )
            .execute(&mut **txn)
            .await
        })
    })
    .await?;
    Ok(HttpResponse::Ok().finish())
}
