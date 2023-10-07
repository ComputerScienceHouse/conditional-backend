use crate::api::{log_query, log_query_as, open_transaction};
use crate::app::AppState;
use crate::schema::api::*;
use crate::schema::db::CommitteeType;

use actix_web::{
    delete, get, post, put,
    web::{Data, Json, Path},
    HttpResponse, Responder,
};
use log::{log, Level};
use sqlx::{query, query_as};

#[utoipa::path(
    context_path="/attendance",
    responses(
        (status = 200, description = "Submit new directorship attendance"),
        (status = 500, description = "Error created by Query"),
        )
    )]
#[post("/directorship")]
pub async fn submit_directorship_attendance(
    state: Data<AppState>,
    body: Json<DirectorshipAttendance>,
) -> impl Responder {
    log!(Level::Info, "POST /attendance/directorship");
    let mut transaction = match open_transaction(&state.db).await {
        Ok(t) => t,
        Err(res) => return res,
    };
    log!(Level::Trace, "Acquired transaction");

    let id: i32;

    match log_query_as(
        query_as!(
            ID,
            "INSERT INTO committee_meetings (committee, \"timestamp\", active, approved)
                VALUES ($1::committees_enum, $2, $3, $4) RETURNING id",
            body.committee as CommitteeType,
            body.timestamp,
            true,
            body.approved
        )
        .fetch_all(&state.db)
        .await,
        Some(transaction),
    )
    .await
    {
        Ok((t, i)) => {
            id = i[0].id;
            transaction = t.unwrap();
        }
        Err(res) => return res,
    }
    log!(Level::Debug, "Inserted directorship into db ID={}", id);

    let frosh_id = vec![id; body.frosh.len()];
    let member_id = vec![id; body.frosh.len()];

    // Add frosh, directorship relation
    match log_query(
        query!(
            "INSERT INTO freshman_committee_attendance (fid, meeting_id)
                SELECT fid, meeting_id
                FROM UNNEST($1::int4[], $2::int4[]) AS a(fid, meeting_id)",
            body.frosh.as_slice(),
            frosh_id.as_slice()
        )
        .fetch_all(&state.db)
        .await
        .map(|_| ()),
        Some(transaction),
    )
    .await
    {
        Ok(t) => transaction = t.unwrap(),
        Err(res) => return res,
    }

    match log_query(
        query!(
            "INSERT INTO member_committee_attendance (uid, meeting_id)
                SELECT uid, meeting_id
                FROM UNNEST($1::TEXT[], $2::int4[]) AS a(uid, meeting_id)",
            body.members.as_slice(),
            member_id.as_slice()
        )
        .fetch_all(&state.db)
        .await
        .map(|_| ()),
        Some(transaction),
    )
    .await
    {
        Ok(t) => transaction = t.unwrap(),
        Err(res) => return res,
    };

    match transaction.commit().await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[utoipa::path(
    context_path="/attendance",
    responses(
        (status = 200, description = "Get all directorships a user has attended", body = [Directorship]),
        (status = 500, description = "Error created by Query"),
        )
    )]
#[get("/directorship/{user}")]
pub async fn get_directorships_by_user(
    path: Path<(String,)>,
    state: Data<AppState>,
) -> impl Responder {
    let (user,) = path.into_inner();
    log!(Level::Info, "GET /attendance/directorship/{}", user);

    if user.chars().next().unwrap().is_numeric() {
        let user: i32 = match user.parse() {
            Ok(user) => user,
            Err(_e) => {
                log!(Level::Warn, "Invalid id");
                return HttpResponse::BadRequest().body("Invalid id");
            }
        };
        match log_query_as(
            query_as!(
                Directorship,
                "SELECT cm.committee AS \"committee:_\",
                        cm.\"timestamp\",
                        ARRAY[]::varchar[] AS members,
                        ARRAY[]::integer[] AS frosh,
                        cm.approved
                    FROM committee_meetings cm
                    LEFT JOIN freshman_committee_attendance fca ON
                        fca.meeting_id = cm.id
                    WHERE cm.approved
                    AND timestamp > $1::timestamp
                    AND fca.fid = $2::int4",
                &state.year_start,
                user
            )
            .fetch_all(&state.db)
            .await,
            None,
        )
        .await
        {
            Ok((_, seminars)) => HttpResponse::Ok().json(seminars),
            Err(e) => return e,
        }
    } else {
        match log_query_as(
            query_as!(
                Directorship,
                "SELECT cm.committee AS \"committee: _\",
                        cm.\"timestamp\",
                        ARRAY[]::varchar[] AS members,
                        ARRAY[]::integer[] AS frosh,
                        cm.approved
                    FROM committee_meetings cm
                    LEFT JOIN member_committee_attendance mca ON
                        mca.meeting_id = cm.id
                    WHERE cm.approved
                    AND timestamp > $1::timestamp
                    AND mca.uid = $2",
                &state.year_start,
                user
            )
            .fetch_all(&state.db)
            .await,
            None,
        )
        .await
        {
            Ok((_, seminars)) => HttpResponse::Ok().json(seminars),
            Err(e) => return e,
        }
    }
}

#[utoipa::path(
    context_path="/attendance",
    responses(
        (status = 200, description = "Get all directorships in the current operating session", body = [Directorship]),
        (status = 500, description = "Error created by Query"),
        )
    )]
#[get("/directorship")]
pub async fn get_directorships(state: Data<AppState>) -> impl Responder {
    log!(Level::Info, "GET /attendance/directorship");
    match query_as!(
        Directorship,
        "SELECT member_seminars.committee AS \"committee: _\",
                member_seminars.timestamp,
                member_seminars.members,
                array_agg(fsa.fid) AS frosh,
                member_seminars.approved
            FROM(
                SELECT ts.id,
                       ts.committee,
                       ts.timestamp,
                       array_agg(msa.uid) AS members,
                       ts.approved
                FROM committee_meetings ts
                INNER JOIN member_committee_attendance msa ON
                    msa.meeting_id = ts.id
                WHERE timestamp > $1
                GROUP BY ts.id, ts.committee, ts.timestamp, ts.approved) AS member_seminars
                INNER JOIN freshman_committee_attendance fsa ON
                    fsa.meeting_id = member_seminars.id
                GROUP BY member_seminars.id,
                    member_seminars.committee,
                    member_seminars.timestamp,
                    member_seminars.members,
                    member_seminars.approved",
        &state.year_start
    )
    .fetch_all(&state.db)
    .await
    {
        Ok(seminars) => HttpResponse::Ok().json(seminars),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[utoipa::path(
    context_path="/attendance",
    responses(
        (status = 200, description = "Delete directorship with a given id"),
        (status = 500, description = "Error created by Query"),
        )
    )]
#[delete("/directorship/{id}")]
pub async fn delete_directorship(_path: Path<(String,)>, _state: Data<AppState>) -> impl Responder {
    return HttpResponse::InternalServerError().body("Not implemented yet");
}

#[utoipa::path(
    context_path="/attendance",
    responses(
        (status = 200, description = "Update directorship"),
        (status = 500, description = "Error created by Query"),
        )
    )]
#[put("/directorship/{id}")]
pub async fn edit_directorship_attendance(
    _path: Path<(String,)>,
    _state: Data<AppState>,
    _body: Json<DirectorshipAttendance>,
) -> impl Responder {
    return HttpResponse::InternalServerError().body("Not implemented yet");
}
