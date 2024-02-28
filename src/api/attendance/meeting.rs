use crate::api::lib::UserError;
use crate::app::AppState;
use crate::auth::{CSHAuth, UserInfo};
use crate::schema::api;
use crate::schema::db;
use crate::schema::db::MeetingType;
use actix_web_validator::Query;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

use actix_web::{
    delete, get, patch, post,
    web::{Data, Json},
    HttpResponse, Responder,
};
use sqlx::{query, query_as, Connection, Pool, Postgres, Transaction};

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct MeetingSubmission {
    /// Type of the meeting
    pub meeting_type: MeetingType,
    /// Date the meeting occured
    pub timestamp: chrono::NaiveDateTime,
    /// Name of the meeting
    pub name: String,
    /// List of User IDs that attended
    pub attendees: Vec<i32>,
}

#[derive(sqlx::Type)]
struct Count {
    count: i64,
}

async fn insert_meeting_attendance<'a>(
    meeting_id: i32,
    uids: Vec<i32>,
    transaction: &mut Transaction<'a, Postgres>,
) -> Result<(), UserError> {
    let meeting_id_vec = vec![meeting_id; uids.len()];
    query!(
        "INSERT INTO om_attendance (uid, om_id)
            SELECT uid, om_id
            FROM UNNEST($1::int4[], $2::int4[]) AS tmp(uid, om_id)
            ON CONFLICT DO NOTHING",
        uids.as_slice(),
        meeting_id_vec.as_slice()
    )
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn get_meetings(
    db: &Pool<Postgres>,
    uuid: String,
    meeting_type: MeetingType,
    start_time: Option<NaiveDateTime>,
) -> Result<Vec<api::Meeting>, UserError> {
    let start_time = start_time.unwrap_or(NaiveDateTime::UNIX_EPOCH);
    Ok(query_as!(
        api::Meeting,
        r#"SELECT
            om.id,
            om.meeting_type as "meeting_type!: MeetingType",
            om.datetime as "timestamp",
            om.name,
            om.approved
        FROM other_meeting om
        LEFT JOIN om_attendance oa
            ON om.id = oa.om_id
        LEFT JOIN "user" u
            ON u.id = oa.uid
        WHERE om.approved
        AND (u.ipa_unique_id = $1::varchar OR u.intro_id = $1::varchar)
        AND om.meeting_type = $2::meeting_type_enum
        AND om.datetime > $3::timestamp
        ORDER BY om.datetime DESC, om.id DESC"#,
        uuid,
        meeting_type as MeetingType,
        &start_time
    )
    .fetch_all(db)
    .await?)
}

/// Submit a directorship/seminar attendance
///
/// Accessible by: CSH members
#[utoipa::path(
    context_path = "/api/attendance",
    tag = "Attendance",
    request_body = MeetingSubmission,
    responses(
        (status = 200, description = "Sucessfully submitted new meeting attendance"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    ),
    security(
        ("csh" = []),
    )
)]
#[post("/meeting", wrap = "CSHAuth::member_only()")]
pub async fn submit_meeting_attendance(
    state: Data<AppState>,
    body: Json<MeetingSubmission>,
) -> Result<impl Responder, UserError> {
    let mut conn = state.db.acquire().await?;
    conn.transaction(|txn| {
        Box::pin(async move {
            let id = query_as!(
                db::ID,
                "INSERT INTO other_meeting (datetime, name, meeting_type, approved)
                        VALUES ($1, $2, $3, $4) RETURNING id",
                body.timestamp,
                body.name,
                body.meeting_type as MeetingType,
                false
            )
            .fetch_one(&mut **txn)
            .await?;
            let members: Vec<i32> = body.attendees.to_vec();
            insert_meeting_attendance(*id, members, &mut *txn).await
        })
    })
    .await?;
    Ok(HttpResponse::Ok().finish())
}

/// Get user's directorships
///
/// Accessible by: CSH and Intro members
#[utoipa::path(
    context_path="/api/attendance",
    tag = "Attendance",
    responses(
        (status = 200, description = "Get all directorships a user has attended", body =Meeting),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    ),
    security(
        ("csh" = []),
        ("intro" = []),
    )
)]
#[get("/meeting/directorship/self", wrap = "CSHAuth::member_and_intro()")]
pub async fn get_user_directorships(
    user: UserInfo,
    state: Data<AppState>,
) -> Result<impl Responder, UserError> {
    let meetings =
        get_meetings(&state.db, user.get_uuid(), MeetingType::Directorship, None).await?;
    Ok(HttpResponse::Ok().json(meetings))
}

/// Get user's seminars
///
/// Accessible by: CSH and Intro members
#[utoipa::path(
    context_path="/api/attendance",
    tag = "Attendance",
    responses(
        (status = 200, description = "Get all seminars a user has attended", body = Meeting),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    ),
    security(
        ("csh" = []),
        ("intro" = []),
    )
)]
#[get("/meeting/seminars/self", wrap = "CSHAuth::member_and_intro()")]
pub async fn get_user_seminars(
    user: UserInfo,
    state: Data<AppState>,
) -> Result<impl Responder, UserError> {
    let meetings = get_meetings(&state.db, user.get_uuid(), MeetingType::Seminar, None).await?;
    Ok(HttpResponse::Ok().json(meetings))
}

#[derive(Deserialize, ToSchema, Validate, IntoParams)]
struct AttendanceHistoryParameters {
    /// Page number (min = 1)
    #[param(minimum = 1)]
    #[validate(range(min = 1))]
    page: i32,
    /// Items per page (min = 1, max = 50)
    #[param(minimum = 1, maximum = 50)]
    #[validate(range(min = 1, max = 50))]
    limit: i32,
    /// Only return meetings before this timestamp
    timestamp: Option<chrono::NaiveDateTime>,
}

/// Get attendance history
///
/// Accessible by: Eboard
#[utoipa::path(
    context_path="/api/attendance",
    tag = "Attendance",
    params(
        AttendanceHistoryParameters
    ),
    responses(
        (status = 200, description = "Get attendances", body = MeetingAttendance),
        (status = 401, description = "Unauthorized"),
        (status = 400, description = "Bad Request"),
        (status = 500, description = "Internal Server Error"),
    ),
    security(
        ("csh" = ["eboard"]),
    )
)]
#[get("/meeting/attendance", wrap = "CSHAuth::eboard_only()")]
pub async fn get_attendance_history(
    state: Data<AppState>,
    body: Query<AttendanceHistoryParameters>,
) -> Result<impl Responder, UserError> {
    #[derive(sqlx::FromRow, Serialize)]
    struct MeetingAttendance {
        meeting: sqlx::types::Json<api::Meeting>,
        attendees: sqlx::types::Json<Vec<db::User>>,
    }
    let attendances = query_as!(
        MeetingAttendance,
        r#"
        SELECT
        json_build_object(
            'id', om.id,
            'meeting_type', om.meeting_type,
            'name', om."name",
            'timestamp', om.datetime,
            'approved', om.approved
        ) AS "meeting!: sqlx::types::Json<api::Meeting>",
        CASE WHEN count(u.id) = 0 THEN '[]' ELSE json_agg(u.*) END AS "attendees!: sqlx::types::Json<Vec<db::User>>"
        FROM other_meeting om
        LEFT JOIN om_attendance oa
            ON om.id = oa.om_id
        LEFT JOIN "user" u
            ON u.id = oa.uid
        WHERE om.datetime < $3::timestamp
        GROUP BY om.id
        ORDER BY om.datetime DESC, om.id DESC
        LIMIT $1::int4
        OFFSET $2::int4
        "#,
        body.limit,
        body.limit * (body.page - 1),
        body.timestamp
    )
    .fetch_all(&state.db)
    .await?;
    Ok(HttpResponse::Ok().json(attendances))
}

#[derive(Deserialize, ToSchema)]
pub struct DeleteMeetingParameters {
    /// Meeting ID
    id: i32,
}

/// Delete a meeting
///
/// Accessible by: Eboard
#[utoipa::path(
    context_path="/api/attendance",
    tag = "Attendance",
    request_body = DeleteMeetingParameters,
    responses(
        (status = 200, description = "Deleted Attendance"),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    ),
    security(
        ("csh" = ["eboard"]),
    )
)]
#[delete("/meeting", wrap = "CSHAuth::evals_only()")]
pub async fn delete_meeting(
    state: Data<AppState>,
    body: Json<DeleteMeetingParameters>,
) -> Result<impl Responder, UserError> {
    let mut conn = state.db.acquire().await?;
    conn.transaction(|txn| {
        Box::pin(async move {
            query!("DELETE FROM other_meeting WHERE id = $1", body.id)
                .execute(&mut **txn)
                .await
        })
    })
    .await?;
    Ok(HttpResponse::Ok().finish())
}

#[derive(Deserialize, ToSchema)]
pub struct ModifyMeetingParameters {
    id: i32,
    add: Option<Vec<i32>>,
    delete: Option<Vec<i32>>,
}

/// Modify a meeting's attendance
///
/// Adding attendance happens before deleting, so if the same user ID is present
/// in both `add` and `delete` lists, it will be deleted.
///
/// Accessible by: Eboard
#[utoipa::path(
    context_path="/api/attendance",
    tag = "Attendance",
    request_body = ModifyMeetingParameters,
    responses(
        (status = 200, description = "Modified Attendance"),
        (status = 401, description = "Unauthorized"),
        (status = 400, description = "Bad Request"),
        (status = 500, description = "Internal Server Error"),
    ),
    security(
        ("csh" = ["eboard"]),
    )
)]
#[patch("/meeting/attendance", wrap = "CSHAuth::eboard_only()")]
pub async fn modify_attendance(
    state: Data<AppState>,
    body: Json<ModifyMeetingParameters>,
) -> Result<impl Responder, UserError> {
    let mut conn = state.db.acquire().await?;
    conn.transaction(|txn| {
        Box::pin(async move {
            if let Some(to_add) = &body.add {
                insert_meeting_attendance(body.id, to_add.to_vec(), &mut *txn).await?;
            }
            if let Some(to_delete) = &body.delete {
                query!(
                    "DELETE FROM om_attendance
                        WHERE om_id = $1 AND uid = ANY ($2::int4[])",
                    body.id,
                    &to_delete
                )
                .execute(&mut **txn)
                .await?;
            }
            Ok::<(), UserError>(())
        })
    })
    .await?;
    Ok(HttpResponse::Ok().finish())
}
