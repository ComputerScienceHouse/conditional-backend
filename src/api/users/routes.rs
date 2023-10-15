use crate::api::{log_query, log_query_as, open_transaction};
use crate::auth::CSHAuth;
use crate::ldap;
use crate::schema::api::{FreshmanUpgrade, ID};
use crate::{app::AppState, schema::api::NewIntroMember};
use actix_web::{
    get, post, put,
    web::{Data, Json, Path},
    HttpResponse, Responder,
};
use log::{log, Level};
use sqlx::{query, query_as};

#[utoipa::path(
    context_path="/api/users",
    responses(
        (status = 200, description = "The number of active voting members"),

        )
    )]
#[get("/voting_count", wrap = "CSHAuth::enabled()")]
pub async fn get_voting_count(state: Data<AppState>) -> impl Responder {
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
    state: Data<AppState>,
    body: Json<NewIntroMember>,
) -> impl Responder {
    let mut transaction = match open_transaction(&state.db).await {
        Ok(t) => t,
        Err(res) => return res,
    };
    log!(Level::Trace, "Acquired transaction");

    let id: i32;

    match log_query_as(
        query_as!(
            ID,
            "INSERT INTO freshman_accounts (name, eval_date, onfloor_status, room_number, \
             signatures_missed, rit_username)
        VALUES ($1::varchar, $2::date, $3, $4::varchar, null, $5::varchar) RETURNING id",
            body.name,
            body.eval_date,
            body.onfloor_status,
            body.room_number,
            body.rit_username
        )
        .fetch_all(&state.db)
        .await,
        Some(transaction),
    )
    .await
    {
        Ok((tx, i)) => {
            transaction = tx.unwrap();
            id = i[0].id;
        }
        Err(res) => return res,
    }
    log!(Level::Debug, "Inserted freshman into db. ID={}", id);
    match transaction.commit().await {
        Ok(_) => HttpResponse::Created().finish(),
        Err(e) => {
            log!(Level::Error, "Transaction failed to commit");
            HttpResponse::InternalServerError().body(e.to_string())
        }
    }
}

#[utoipa::path(
    context_path="/api/users",
    responses(
        (status = 200, description = "Freshman user successfully converted to member"),
        )
    )]
#[put("/", wrap = "CSHAuth::evals_only()")]
pub async fn convert_freshman_user(
    state: Data<AppState>,
    body: Json<FreshmanUpgrade>,
) -> impl Responder {
    let mut transaction = match open_transaction(&state.db).await {
        Ok(t) => t,
        Err(res) => return res,
    };
    log!(Level::Trace, "Acquired transaction");

    // Migrate directorship attendance
    transaction = match log_query(
        query!(
            "WITH dir_attendance AS (
                DELETE FROM freshman_committee_attendance fca
                WHERE fid=$1::int4
                RETURNING $2::varchar AS uid, fca.meeting_id
            ) INSERT INTO member_committee_attendance (uid, meeting_id)
            SELECT * FROM dir_attendance",
            body.fid,
            body.uid,
        )
        .fetch_all(&state.db)
        .await
        .map(|_| ()),
        Some(transaction),
    )
    .await
    {
        Ok(tx) => tx.unwrap(),
        Err(res) => return res,
    };

    // Migrate house meeting attendance
    transaction = match log_query(
        query!(
            "WITH hm_attendance AS (
                DELETE FROM freshman_hm_attendance fha 
                WHERE fid=$1::int4
                RETURNING $2::varchar AS uid, fha.meeting_id, fha.excuse, fha.attendance_status
            ) INSERT INTO member_hm_attendance (uid, meeting_id, excuse, attendance_status)
            SELECT * FROM hm_attendance",
            body.fid,
            body.uid,
        )
        .fetch_all(&state.db)
        .await
        .map(|_| ()),
        Some(transaction),
    )
    .await
    {
        Ok(tx) => tx.unwrap(),
        Err(res) => return res,
    };

    // Migrate seminar attendance
    transaction = match log_query(
        query!(
            "WITH sem_attendance AS (
                DELETE FROM freshman_seminar_attendance fsa 
                WHERE fid=$1::int4
                RETURNING $2::varchar AS uid, fsa.seminar_id
            ) INSERT INTO member_seminar_attendance (uid, seminar_id)
            SELECT * FROM sem_attendance",
            body.fid,
            body.uid,
        )
        .fetch_all(&state.db)
        .await
        .map(|_| ()),
        Some(transaction),
    )
    .await
    {
        Ok(tx) => tx.unwrap(),
        Err(res) => return res,
    };

    // Remove freshman from any batch tables for deletion
    // cascading delete scares me
    transaction = match log_query(
        query!(
            "WITH fbps_deleted AS (
                DELETE FROM freshman_batch_pulls fbp 
                WHERE fbp.fid=$1::int4
            ),
            fbus_deleted AS (
                DELETE FROM freshman_batch_users fbu  
                WHERE fbu.fid=$1::int4
            )
            DELETE FROM freshman_accounts fa WHERE fa.id=$1::int4",
            body.fid,
        )
        .fetch_all(&state.db)
        .await
        .map(|_| ()),
        Some(transaction),
    )
    .await
    {
        Ok(tx) => tx.unwrap(),
        Err(res) => return res,
    };

    match transaction.commit().await {
        Ok(_) => HttpResponse::Created().finish(),
        Err(e) => {
            log!(Level::Error, "Transaction failed to commit");
            HttpResponse::InternalServerError().body(e.to_string())
        }
    }
}
