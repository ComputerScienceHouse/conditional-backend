use crate::api::lib::{log_query, log_query_as, open_transaction, UserError};
use crate::app::AppState;
use crate::auth::CSHAuth;
use crate::schema::api::User;
use crate::schema::db::{MeetingType, ID};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use actix_web::{
    delete,
    dev::Extensions,
    get, post, put,
    web::{Data, Json, Path},
    HttpResponse, Responder,
};
use log::{log, Level};
use sqlx::{query, query_as, Pool, Postgres, Transaction};

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct MeetingSubmission {
    /// Type of the meeting
    pub meeting_type: Option<MeetingType>,
    /// Date the meeting occured
    pub timestamp: chrono::NaiveDateTime,
    /// Name of the meeting
    pub name: String,
    /// List of User IDs that attended
    pub attendees: Vec<i32>,
}

// async fn delete_directorship_attendance<'a>(
//     id: i32,
//     mut transaction: Transaction<'a, Postgres>,
//     db: &Pool<Postgres>,
// ) -> Result<Transaction<'a, Postgres>, HttpResponse> {
//     match log_query(
//         query!(
//             "DELETE FROM freshman_committee_attendance WHERE meeting_id = $1",
//             id
//         )
//         .execute(db)
//         .await
//         .map(|_| ()),
//         Some(transaction),
//     )
//     .await
//     {
//         Ok(tx) => {
//             transaction = tx.unwrap();
//         }
//         Err(res) => return Err(res),
//     };
//     match log_query(
//         query!(
//             "DELETE FROM member_committee_attendance WHERE meeting_id = $1",
//             id
//         )
//         .execute(db)
//         .await
//         .map(|_| ()),
//         Some(transaction),
//     )
//     .await
//     {
//         Ok(tx) => {
//             transaction = tx.unwrap();
//         }
//         Err(res) => return Err(res),
//     };
//     log!(Level::Trace, "Deleted directorship attendance");
//     Ok(transaction)
// }

/// Submit a directorship/seminar attendance.
#[utoipa::path(
    context_path = "/api/attendance",
    tag = "Attendance",
    responses(
        (status = 200, description = "Sucessfully submitted new meeting attendance"),
        (status = 500, description = "Internal Server Error"),
        )
    )]
#[post("/meeting", wrap = "CSHAuth::enabled()")]
pub async fn submit_meeting_attendance(
    meeting_type: Path<String>,
    state: Data<AppState>,
    body: Json<MeetingSubmission>,
) -> Result<impl Responder, UserError> {
    log!(Level::Info, "POST /attendance/{meeting_type}");
    // return Ok(HttpResponse::Ok().finish());
    let mut transaction = open_transaction(&state.db).await?;
    let id = query_as!(
        ID,
        "INSERT INTO other_meeting (datetime, name, meeting_type, approved)
                VALUES ($1, $2, $3, $4) RETURNING id",
        body.timestamp,
        body.name,
        body.meeting_type.unwrap() as MeetingType,
        false
    )
    .fetch_one(&mut *transaction)
    .await?;
    log!(Level::Trace, "Inserted directorship into db ID={}", id);
    // Add attendance
    let members: Vec<i32> = body.attendees.iter().map(|id| *id).collect();
    let meeting_id_vec = vec![*id; members.len()];
    query!(
        "INSERT INTO om_attendance (uid, om_id)
            SELECT uid, om_id
            FROM UNNEST($1::int4[], $2::int4[]) AS tmp(uid, om_id)",
        members.as_slice(),
        meeting_id_vec.as_slice()
    )
    .execute(&mut *transaction)
    .await?;
    log!(Level::Trace, "Added directorship attendance");
    transaction.rollback().await?;
    Ok(HttpResponse::Ok().finish())
}

// #[utoipa::path(
//     context_path="/api/attendance",
//     tag = "Attendance",
//     responses(
//         (status = 200, description = "Get all directorships a user has attended", body = [Meeting]),
//         (status = 500, description = "Internal Server Error"),
//         )
//     )]
// #[get("/meeting/directorship/{user}", wrap = "CSHAuth::enabled()")]
// pub async fn get_directorships_by_user(
//     path: Path<(i32,)>,
//     state: Data<AppState>,
// ) -> impl Responder {
//     let (user,) = path.into_inner();
//     if user.chars().next().unwrap().is_numeric() {
//         let user: i32 = match user.parse() {
//             Ok(user) => user,
//             Err(_e) => {
//                 log!(Level::Warn, "Invalid id");
//                 return HttpResponse::BadRequest().body("Invalid id");
//             }
//         };
//         match log_query_as(
//             query_as!(
//                 Directorship,
//                 "SELECT cm.committee AS \"committee:_\",
//                         cm.\"timestamp\",
//                         ARRAY[]::varchar[] AS members,
//                         ARRAY[]::integer[] AS frosh,
//                         cm.approved
//                     FROM committee_meetings cm
//                     LEFT JOIN freshman_committee_attendance fca ON
//                         fca.meeting_id = cm.id
//                     WHERE cm.approved
//                     AND timestamp > $1::timestamp
//                     AND fca.fid = $2::int4",
//                 &state.year_start,
//                 user
//             )
//             .fetch_all(&state.db)
//             .await,
//             None,
//         )
//         .await
//         {
//             Ok((_, seminars)) => HttpResponse::Ok().json(seminars),
//             Err(e) => return e,
//         }
//     } else {
//         match log_query_as(
//             query_as!(
//                 Directorship,
//                 "SELECT cm.committee AS \"committee: _\",
//                         cm.\"timestamp\",
//                         ARRAY[]::varchar[] AS members,
//                         ARRAY[]::integer[] AS frosh,
//                         cm.approved
//                     FROM committee_meetings cm
//                     LEFT JOIN member_committee_attendance mca ON
//                         mca.meeting_id = cm.id
//                     WHERE cm.approved
//                     AND timestamp > $1::timestamp
//                     AND mca.uid = $2",
//                 &state.year_start,
//                 user
//             )
//             .fetch_all(&state.db)
//             .await,
//             None,
//         )
//         .await
//         {
//             Ok((_, seminars)) => HttpResponse::Ok().json(seminars),
//             Err(e) => return e,
//         }
//     }
// }

// #[utoipa::path(
//     context_path="/api/attendance",
//     responses(
//         (status = 200, description = "Get all directorships in the current operating session", body = [Directorship]),
//         (status = 500, description = "Error created by Query"),
//         )
//     )]
// #[get("/directorship", wrap = "CSHAuth::enabled()")]
// pub async fn get_directorships(state: Data<AppState>) -> impl Responder {
//     match query_as!(
//         Directorship,
//         "SELECT member_seminars.committee AS \"committee: _\",
//                 member_seminars.timestamp,
//                 member_seminars.members,
//                 array_agg(fsa.fid) AS frosh,
//                 member_seminars.approved
//             FROM(
//                 SELECT ts.id,
//                        ts.committee,
//                        ts.timestamp,
//                        array_agg(msa.uid) AS members,
//                        ts.approved
//                 FROM committee_meetings ts
//                 INNER JOIN member_committee_attendance msa ON
//                     msa.meeting_id = ts.id
//                 WHERE timestamp > $1
//                 GROUP BY ts.id, ts.committee, ts.timestamp, ts.approved) AS member_seminars
//                 INNER JOIN freshman_committee_attendance fsa ON
//                     fsa.meeting_id = member_seminars.id
//                 GROUP BY member_seminars.id,
//                     member_seminars.committee,
//                     member_seminars.timestamp,
//                     member_seminars.members,
//                     member_seminars.approved",
//         &state.year_start
//     )
//     .fetch_all(&state.db)
//     .await
//     {
//         Ok(seminars) => HttpResponse::Ok().json(seminars),
//         Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
//     }
// }

// #[utoipa::path(
//     context_path="/api/attendance",
//     responses(
//         (status = 200, description = "Delete directorship with a given id"),
//         (status = 500, description = "Error created by Query"),
//         )
//     )]
// #[delete("/directorship/{id}", wrap = "CSHAuth::eboard_only()")]
// pub async fn delete_directorship(path: Path<(String,)>, state: Data<AppState>) -> impl Responder {
//     let (id,) = path.into_inner();
//     let id = match id.parse::<i32>() {
//         Ok(id) => id,
//         Err(_e) => {
//             log!(Level::Warn, "Invalid id");
//             return HttpResponse::BadRequest().body("Invalid id");
//         }
//     };
//     let mut transaction = match open_transaction(&state.db).await {
//         Ok(t) => t,
//         Err(res) => return res,
//     };
//     log!(Level::Trace, "Acquired transaction");
//     match delete_directorship_attendance(id, transaction, &state.db).await {
//         Ok(tx) => {
//             transaction = tx;
//         }
//         Err(res) => return res,
//     };
//     match log_query(
//         query!("DELETE FROM committee_meetings WHERE id = $1", id)
//             .execute(&state.db)
//             .await
//             .map(|_| ()),
//         Some(transaction),
//     )
//     .await
//     {
//         Ok(tx) => {
//             transaction = tx.unwrap();
//         }
//         Err(res) => return res,
//     }
//     log!(Level::Trace, "Finished deleting directorship");
//     match transaction.commit().await {
//         Ok(_) => HttpResponse::Ok().body(""),
//         Err(e) => {
//             log!(Level::Error, "Transaction failed to commit");
//             HttpResponse::InternalServerError().body(e.to_string())
//         }
//     }
// }

// #[utoipa::path(
//     context_path="/api/attendance",
//     responses(
//         (status = 200, description = "Update directorship"),
//         (status = 500, description = "Error created by Query"),
//         )
//     )]
// #[put("/directorship/{id}", wrap = "CSHAuth::eboard_only()")]
// pub async fn edit_directorship_attendance(
//     path: Path<(String,)>,
//     state: Data<AppState>,
//     body: Json<DirectorshipAttendance>,
// ) -> impl Responder {
//     let (id,) = path.into_inner();
//     let id = match id.parse::<i32>() {
//         Ok(id) => id,
//         Err(_e) => {
//             log!(Level::Warn, "Invalid id");
//             return HttpResponse::BadRequest().body("Invalid id");
//         }
//     };
//     let mut transaction = match open_transaction(&state.db).await {
//         Ok(t) => t,
//         Err(res) => return res,
//     };
//     log!(Level::Trace, "Acquired transaction");

//     match delete_directorship_attendance(id, transaction, &state.db).await {
//         Ok(tx) => {
//             transaction = tx;
//         }
//         Err(e) => return e,
//     };
//     match create_directorship_attendance(id, body, transaction, &state.db).await {
//         Ok(tx) => {
//             transaction = tx;
//         }
//         Err(e) => return e,
//     };
//     match transaction.commit().await {
//         Ok(_) => HttpResponse::Ok().body(""),
//         Err(e) => {
//             log!(Level::Error, "Transaction failed to commit");
//             HttpResponse::InternalServerError().body(e.to_string())
//         }
//     }
// }
