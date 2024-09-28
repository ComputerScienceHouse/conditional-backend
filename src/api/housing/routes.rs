use actix_web::{
    get, post,
    web::{Data, Json},
    HttpResponse, Responder,
};
use sqlx::{query, query_as};

use crate::{api::lib::UserError, app::AppState, auth_service::CSHAuth, schema::db};

#[utoipa::path(
    context_path = "/housing/queue",
    tag = "Housing",
    responses(
        (status = 200, description = "Get all members in housing queue", body = Vec<ID>),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    ),
    security(
        ("csh" = []),
        ("intro" = []),
    )
)]
#[get("/queue", wrap = "CSHAuth::member_and_intro()")]
pub async fn get_housing_queue(state: Data<AppState>) -> Result<impl Responder, UserError> {
    let queue = query_as!(
        db::ID,
        "select uid as id from housing_queue order by datetime_added"
    )
    .fetch_all(&state.db)
    .await?;

    Ok(HttpResponse::Ok().json(queue))
}

#[utoipa::path(
    context_path = "/housing/queue",
    tag = "Housing",
    responses(
        (status = 200, description = "Get all members in housing queue", body = Vec<ID>),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    ),
)]
#[post("/queue", wrap = "CSHAuth::evals_only()")]
pub async fn add_to_housing_queue(
    state: Data<AppState>,
    body: Json<i32>,
) -> Result<impl Responder, UserError> {
    let uid = body.into_inner();
    query!(
        "insert into housing_queue values($1, now()::timestamp)",
        uid
    )
    .execute(&state.db)
    .await?;

    Ok(HttpResponse::NoContent().finish())
}
