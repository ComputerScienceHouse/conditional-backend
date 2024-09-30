use actix_web::{
    delete, get, post,
    web::{Data, Json},
    HttpResponse, Responder,
};
use sqlx::{query, query_as, Postgres};

use crate::{
    api::lib::UserError,
    app::AppState,
    auth_service::CSHAuth,
    schema::api::{Room, User},
};

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
        User,
        "select uid, name, rit_username, csh_username from housing_queue left join \"user\" u on \
         uid = u.id order by datetime_added"
    )
    .fetch_all(&state.db)
    .await?;

    Ok(HttpResponse::Ok().json(queue))
}

#[utoipa::path(
    context_path = "/housing/queue",
    tag = "Housing",
    responses(
        (status = 200, description = "Add a user to housing queue", body = Vec<ID>),
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

#[utoipa::path(
    context_path = "/housing/queue",
    tag = "Housing",
    responses(
        (status = 200, description = "Remove a user from housing queue", body = Vec<ID>),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    ),
)]
#[delete("/queue", wrap = "CSHAuth::evals_only()")]
pub async fn remove_from_housing_queue(
    state: Data<AppState>,
    body: Json<i32>,
) -> Result<impl Responder, UserError> {
    let uid = body.into_inner();
    query!("delete from housing_queue where uid = $1", uid)
        .execute(&state.db)
        .await?;

    Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(
    context_path = "/housing/rooms",
    tag = "Housing",
    responses(
        (status = 200, description = "Get all rooms", body = Vec<Room>),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    ),
)]
#[get("/rooms", wrap = "CSHAuth::member_only()")]
pub async fn get_rooms(state: Data<AppState>) -> Result<impl Responder, UserError> {
    let (onfloor_members, onfloor_rooms): (Vec<_>, Vec<_>) = state
        .ldap
        .get_onfloor_members()
        .await
        .map_err(|_| UserError::ServerError)?
        .into_iter()
        .filter(|l| l.room_number.is_some())
        .map(|l| (l.rit_username, l.room_number.unwrap()))
        .unzip();

    let rooms = query_as::<Postgres, Room>(
        "select array_agg(id order by id) as id, array_agg(name order by id) as name, \
         array_agg(rit_username order by id) as rit_username, array_agg(csh_username order by id) \
         as csh_username, room_number from (select unnest($1::varchar[]) as username, \
         unnest($2::int4[]) as room_number union all select rit_username, room from \
         freshman_rooms left join \"user\" on uid = id) u left join \"user\" on username = \
         rit_username group by room_number",
    )
    .bind(Some(onfloor_members.as_slice()))
    .bind(Some(onfloor_rooms.as_slice()))
    .fetch_all(&state.db)
    .await?;

    Ok(HttpResponse::Ok().json(rooms))
}
