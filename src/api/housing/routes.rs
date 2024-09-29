use actix_web::{
    delete, get, post,
    web::{Data, Json},
    HttpResponse, Responder,
};
use sqlx::{query, query_as};

use crate::{
    api::lib::UserError,
    app::AppState,
    auth_service::CSHAuth,
    schema::{api::Room, db},
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

fn merge_rooms(mut a: Vec<Room>, b: Vec<Room>) -> Vec<Room> {
    a.extend(b);
    a.sort();

    let mut rooms = Vec::new();

    for room in a {
        if rooms
            .last()
            .map(|r: &Room| r.number)
            .is_some_and(|n| n == room.number)
        {
            let r = rooms.last_mut().unwrap();
            r.users = r
                .users
                .as_mut()
                .map(|u| {
                    u.extend(room.users.unwrap_or_default());
                    u
                })
                .cloned();
            r.names = r
                .names
                .as_mut()
                .map(|u| {
                    u.extend(room.names.unwrap_or_default());
                    u
                })
                .cloned();
        } else {
            rooms.push(room);
        }
    }

    Vec::new()
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
pub async fn get_rooms(state: Data<AppState>) -> Result<impl Responder, UserError> {
    let freshman_rooms = query_as!(
        Room,
        "select array_agg(uid order by uid) as users, array_agg(name order by uid) as names, room \
         as number from freshman_rooms left join \"user\" on uid = id group by room"
    )
    .fetch_all(&state.db)
    .await?;

    let (onfloor_members, onfloor_rooms): (Vec<_>, Vec<_>) = state
        .ldap
        .get_onfloor_members()
        .await
        .map_err(|_| UserError::ServerError)?
        .into_iter()
        .filter(|l| l.room_number.is_some())
        .map(|l| (l.uid, l.room_number.unwrap()))
        .unzip();

    let member_rooms = query_as!(
        Room,
        "select array_agg(id order by id) as users, array_agg(name order by id) as names, room as \
         \"number!\" from (select unnest($1::varchar[]) as username, unnest($2::int4[]) as room) \
         as u left join \"user\" on u.username = \"user\".csh_username where room is not null \
         group by room",
        Some(onfloor_members.as_slice()),
        Some(onfloor_rooms.as_slice()),
    )
    .fetch_all(&state.db)
    .await?;

    let rooms = merge_rooms(freshman_rooms, member_rooms);

    Ok(HttpResponse::Ok().json(rooms))
}
