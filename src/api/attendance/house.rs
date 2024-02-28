use actix_web::{
    get, post, put,
    web::{Data, Json},
    HttpResponse, Responder,
};
use chrono::{Datelike, NaiveDate, Utc};
use serde::Serialize;
use sqlx::{query, query_as, Connection};

use crate::{
    api::lib::UserError,
    app::AppState,
    auth::{CSHAuth, UserInfo},
    ldap::get_group_members_exact,
    schema::{
        api::*,
        db::{AttendanceStatus, ID},
    },
};

#[derive(sqlx::Type, Serialize)]
struct Absences {
    uid: i32,
    count: Option<i64>,
}

#[derive(sqlx::Type, Serialize)]
struct DateWrapper {
    date: NaiveDate,
}

#[derive(sqlx::Type, Serialize)]
struct AbsenceWrapper {
    date: NaiveDate,
    excuse: Option<String>,
}

#[utoipa::path(
    context_path = "/api/attendance",
    tag = "Attendance",
    request_body = HouseAttendance,
    responses(
        (status = 200, description = "Success"),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    ),
    security(
        ("csh" = ["eboard"]),
    )
)]
#[post("/house", wrap = "CSHAuth::evals_only()")]
pub async fn submit_hm_attendance(
    state: Data<AppState>,
    body: Json<HouseAttendance>,
) -> Result<impl Responder, UserError> {
    let mut conn = state.db.acquire().await?;
    conn.transaction(|txn| {
        Box::pin(async move {
            let id = query_as!(
                ID,
                "INSERT INTO house_meeting(date) VALUES ($1) RETURNING id",
                body.date
            )
            .fetch_one(&mut **txn)
            .await
            .unwrap();
            let id_vec = vec![id.id; body.attendees.len()];
            let names: Vec<i32> = body.attendees.iter().map(|a| a.uid).collect();
            let statuses: Vec<AttendanceStatus> =
                body.attendees.iter().map(|a| a.att_status).collect();

            query!(
                "INSERT INTO hm_attendance (uid, house_meeting_id, attendance_status) SELECT uid, \
                 house_meeting_id, attendance_status as \"attendance_status: AttendanceStatus\" \
                 FROM UNNEST($1::int4[], $2::int4[], $3::hm_attendance_status_enum[]) as a(uid, \
                 house_meeting_id, attendance_status)",
                names.as_slice(),
                id_vec.as_slice(),
                statuses.as_slice() as &[AttendanceStatus]
            )
            .execute(&mut **txn)
            .await
        })
    })
    .await?;
    Ok(HttpResponse::Ok().finish())
}

#[utoipa::path(
    context_path = "/api/attendance",
    tag = "Attendance",
    responses(
        (status = 200, description = "Success", body = Vec<Absences>),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    ),
    security(
        ("csh" = []),
    )
)]
#[get("/house/users", wrap = "CSHAuth::member_only()")]
pub async fn count_hm_absences(state: Data<AppState>) -> Result<impl Responder, UserError> {
    let now = Utc::now();
    let users = get_group_members_exact(&state.ldap, "active").await;
    let usernames: Vec<_>;
    if let Ok(members) = users {
        usernames = members.into_iter().map(|m| m.rit_username).collect();
    } else {
        log::error!("LDAP is unresponsive");
        return Err(UserError::ServerError);
    }
    let mut conn = state.db.acquire().await?;
    let counts = conn
        .transaction(|txn| {
            Box::pin(async move {
                let uids: Vec<_> = query_as!(
                    ID,
                    "select id from \"user\" where rit_username in (select unnest($1::varchar[]))",
                    usernames.as_slice()
                )
                .fetch_all(&mut **txn)
                .await?
                .iter_mut()
                .map(|x| x.id)
                .collect();

                query_as!(
                    Absences,
                    "select uid, count(*) from hm_attendance hma left join house_meeting hm on \
                     hma.house_meeting_id = hm.id where attendance_status = 'Absent' and date > \
                     $1 group by uid having uid in (select unnest($2::int4[]))",
                    if now.month() > 5 {
                        NaiveDate::from_ymd_opt(now.year(), 6, 1).unwrap()
                    } else {
                        NaiveDate::from_ymd_opt(now.year() - 1, 6, 1).unwrap()
                    },
                    uids.as_slice()
                )
                .fetch_all(&mut **txn)
                .await
            })
        })
        .await?;
    Ok(HttpResponse::Ok().json(counts))
}

#[utoipa::path(
    context_path = "/api/attendance",
    tag = "Attendance",
    responses(
        (status = 200, description = "Success"),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    ),
    security(
        ("csh" = []),
        ("intro" = []),
    )
)]
#[get("/house", wrap = "CSHAuth::member_and_intro()")]
pub async fn get_hm_absences_by_user(
    user: UserInfo,
    state: Data<AppState>,
) -> Result<impl Responder, UserError> {
    // INFO: leaving this here in case I need to get a User from a uid
    // let user = query_as!(User, "SELECT id uid, name, rit_username, csh_username,
    // is_csh, is_intro FROM \"user\" WHERE id = $1",
    // user.get_uid(&state.db).await?).fetch_one(&state.db).await?;
    let now = Utc::now();
    let hms = query_as!(
        DateWrapper,
        "select date from house_meeting where date > $1 and id IN (SELECT house_meeting_id FROM \
         hm_attendance WHERE uid = $2 AND attendance_status = 'Absent')",
        if now.month() > 5 {
            NaiveDate::from_ymd_opt(now.year(), 6, 1).unwrap()
        } else {
            NaiveDate::from_ymd_opt(now.year() - 1, 6, 1).unwrap()
        },
        user.get_uid(&state.db).await?
    )
    .fetch_all(&state.db)
    .await?;
    Ok(HttpResponse::Ok().json(hms))
}

#[utoipa::path(
    context_path = "/api/attendance",
    tag = "Attendance",
    responses(
        (status = 200, description = "Success", body = Vec<AbsenceWrapper>),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    ),
    params(
        ("uid" = String, Path, description = "User"),
    ),
    security(
        ("csh" = ["eboard"]),
    )
)]
#[get("/house/evals/{uid}", wrap = "CSHAuth::evals_only()")]
pub async fn get_hm_attendance_by_user_evals(
    state: Data<AppState>,
    path: actix_web::web::Path<i32>,
) -> Result<impl Responder, UserError> {
    println!("|{:?}|", path);
    let now = Utc::now();
    let hms = query_as!(
        AbsenceWrapper,
        "select date, excuse from hm_attendance left join house_meeting on \
         hm_attendance.house_meeting_id = house_meeting.id where date > $1 and uid = $2 AND \
         attendance_status != 'Attended'",
        if now.month() > 5 {
            NaiveDate::from_ymd_opt(now.year(), 6, 1).unwrap()
        } else {
            NaiveDate::from_ymd_opt(now.year() - 1, 6, 1).unwrap()
        },
        path.into_inner()
    )
    .fetch_all(&state.db)
    .await?;
    Ok(HttpResponse::Ok().json(hms))
}

#[utoipa::path(
    context_path = "/api/attendance",
    tag = "Attendance",
    request_body = HouseMeetingAttendanceUpdate,
    responses(
        (status = 200, description = "Success"),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    ),
    security(
        ("csh" = ["eboard"]),
    )
)]
#[put("/house")]
pub async fn modify_hm_attendance(
    state: Data<AppState>,
    body: Json<HouseAttendanceUpdate>,
) -> Result<impl Responder, UserError> {
    let mut conn = state.db.acquire().await?;
    conn.transaction(|txn| {
        Box::pin(async move {
            query!(
                "UPDATE hm_attendance SET attendance_status = $1, excuse = $2 WHERE uid = $3 AND \
                 house_meeting_id = $4",
                body.att_status as AttendanceStatus,
                body.excuse,
                body.uid,
                body.meeting_id
            )
            .execute(&mut **txn)
            .await
        })
    })
    .await?;
    Ok(HttpResponse::Ok().finish())
}
