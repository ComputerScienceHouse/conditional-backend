use actix_web::{
    get, post, put,
    web::{Data, Json, Path},
    HttpResponse, Responder,
};
use chrono::NaiveDate;
use log::{log, Level};
use sqlx::{query, query_as};

use crate::{
    api::{log_query, log_query_as, open_transaction},
    app::AppState,
    schema::{api::*, db::AttendanceStatus},
};

#[utoipa::path(context_path="/attendance", responses((status = 200, description = "Submit new house meeting attendance"),(status = 500, description = "Error created by Query"),))]
#[post("/house")]
pub async fn submit_hm_attendance(
    state: Data<AppState>,
    body: Json<HouseAttendance>,
) -> impl Responder {
    log!(Level::Info, "POST /attendance/house");
    let mut transaction = match open_transaction(&state.db).await {
        Ok(t) => t,
        Err(res) => return res,
    };

    let id: i32;
    match log_query_as(
        query_as!(
            ID,
            "INSERT INTO house_meetings(date, active) VALUES ($1, true) RETURNING id",
            body.date
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
    log!(Level::Trace, "created new house meeting");

    let frosh_id = vec![id; body.frosh.len()];
    let member_id = vec![id; body.members.len()];
    let frosh_names: Vec<i32> = body.frosh.iter().map(|a| a.name).collect();
    let frosh_statuses: Vec<AttendanceStatus> = body.frosh.iter().map(|a| a.att_status).collect();
    let member_names: Vec<i32> = body.frosh.iter().map(|a| a.name).collect();
    let member_statuses: Vec<AttendanceStatus> = body.frosh.iter().map(|a| a.att_status).collect();

    match log_query(
        query!("INSERT INTO freshman_hm_attendance (fid, meeting_id, attendance_status) SELECT fid, meeting_id, attendance_status as \"attendance_status: AttendanceStatus\" FROM UNNEST($1::int4[], $2::int4[], $3::attendance_enum[]) as a(fid, meeting_id, attendance_status)", frosh_names.as_slice(), frosh_id.as_slice(), frosh_statuses.as_slice() as &[AttendanceStatus])
        .execute(&state.db).await.map(|_| ()), Some(transaction)).await {
        Ok(tx) => transaction = tx.unwrap(),
        Err(res) => return res,
    }

    match log_query(
        query!("INSERT INTO member_hm_attendance (uid, meeting_id, attendance_status) SELECT uid, meeting_id, attendance_status as \"attendance_status: AttendanceStatus\" FROM UNNEST($1::int4[], $2::int4[], $3::attendance_enum[]) as a(uid, meeting_id, attendance_status)", member_names.as_slice(), member_id.as_slice(), member_statuses.as_slice() as &[AttendanceStatus])
        .execute(&state.db).await.map(|_| ()), Some(transaction)).await {
        Ok(tx) => transaction = tx.unwrap(),
        Err(res) => return res,
    }
    log!(Level::Trace, "added attendance to house meeting");

    // Commit transaction
    match transaction.commit().await {
        Ok(_) => HttpResponse::Ok().body(""),
        Err(e) => {
            log!(Level::Error, "Transaction failed to commit");
            HttpResponse::InternalServerError().body(e.to_string())
        }
    }
}

#[utoipa::path(context_path="/attendance", responses((status = 200, description = "Get house meetings missed for a given user", body = [NaiveDate]),(status = 400, description = "Invalid user"),(status = 500, description = "Error created by Query"),))]
#[get("/house/{user}")]
pub async fn get_hm_absences_by_user(
    path: Path<(String,)>,
    state: Data<AppState>,
) -> impl Responder {
    let (user,) = path.into_inner();
    log!(Level::Info, "GET /attendance/house/{user}");

    if user.chars().next().unwrap().is_numeric() {
        let user: i32 = match user.parse() {
            Ok(user) => user,
            Err(_) => {
                log!(Level::Warn, "Invalid id");
                return HttpResponse::BadRequest().body("Invalid id");
            }
        };
        match log_query_as(query_as!(Date, "SELECT date FROM house_meetings WHERE date > $1 AND id IN (SELECT meeting_id FROM freshman_hm_attendance WHERE fid = $2 AND attendance_status = 'Absent')", NaiveDate::from(state.year_start), user).fetch_all(&state.db).await, None).await {
            Ok((_, hms)) => HttpResponse::Ok().json(hms),
            Err(e) => return e,
        }
    } else {
        match log_query_as(query_as!(Date, "SELECT date FROM house_meetings WHERE date > $1 AND id IN (SELECT meeting_id FROM member_hm_attendance WHERE uid = $2 AND attendance_status = 'Absent')", NaiveDate::from(state.year_start), user).fetch_all(&state.db).await, None).await {
            Ok((_, hms)) => HttpResponse::Ok().json(hms),
            Err(e) => return e,
        }
    }
}

#[utoipa::path(context_path="/attendance", responses((status = 200, description = "Get house meetings not attended for a given user", body = [NaiveDate]),(status = 400, description = "Invalid user"),(status = 500, description = "Error created by Query"),))]
#[get("/house/evals/{user}")]
pub async fn get_hm_attendance_by_user_evals(
    path: Path<(String,)>,
    state: Data<AppState>,
) -> impl Responder {
    let (user,) = path.into_inner();
    log!(Level::Info, "GET /attendance/house/evals/{user}");

    if user.chars().next().unwrap().is_numeric() {
        let user: i32 = match user.parse() {
            Ok(user) => user,
            Err(_) => {
                log!(Level::Warn, "Invalid id");
                return HttpResponse::BadRequest().body("Invalid id");
            }
        };
        match log_query_as(query_as!(Date, "SELECT date FROM house_meetings WHERE date > $1 AND id IN (SELECT meeting_id FROM freshman_hm_attendance WHERE fid = $2 AND attendance_status != 'Attended')", NaiveDate::from(state.year_start), user).fetch_all(&state.db).await, None).await {
            Ok((_, hms)) => HttpResponse::Ok().json(hms),
            Err(e) => return e,
        }
    } else {
        match log_query_as(query_as!(Date, "SELECT date FROM house_meetings WHERE date > $1 AND id IN (SELECT meeting_id FROM member_hm_attendance WHERE uid = $2 AND attendance_status != 'Attended')", NaiveDate::from(state.year_start), user).fetch_all(&state.db).await, None).await {
            Ok((_, hms)) => HttpResponse::Ok().json(hms),
            Err(e) => return e,
        }
    }
}

#[utoipa::path(context_path="/attendance", responses((status = 200, description = "Modify attendance for a given user at a given house meeting"),(status = 400, description = "Invalid user"),(status = 500, description = "Error created by Query"),))]
#[put("/house/{date}/{user}")]
pub async fn modify_hm_attendance(
    path: Path<(NaiveDate, String)>,
    state: Data<AppState>,
    body: Json<AttendanceStatus>,
) -> impl Responder {
    let (date, user) = path.into_inner();
    log!(Level::Info, "PUT /attendance/house/{date}/{user}");
    let new_status = body.into_inner();

    let mut transaction = match open_transaction(&state.db).await {
        Ok(t) => t,
        Err(res) => return res,
    };

    if user.chars().next().unwrap().is_numeric() {
        let user: i32 = match user.parse() {
            Ok(user) => user,
            Err(_) => {
                log!(Level::Warn, "Invalid id");
                return HttpResponse::BadRequest().body("Invalid id");
            }
        };
        match log_query(query!("UPDATE freshman_hm_attendance SET attendance_status = $1 WHERE fid = $2 AND id IN (SELECT id FROM house_meetings WHERE date > $3)", new_status as AttendanceStatus, user, date).execute(&state.db).await.map(|_| ()), Some(transaction)).await {
            Ok(tx) => transaction = tx.unwrap(),
            Err(res) => return res,
        }
    } else {
        match log_query(query!("UPDATE member_hm_attendance SET attendance_status = $1 WHERE uid = $2 AND id IN (SELECT id FROM house_meetings WHERE date > $3)", new_status as AttendanceStatus, user, date).execute(&state.db).await.map(|_| ()), Some(transaction)).await {
            Ok(tx) => transaction = tx.unwrap(),
            Err(res) => return res,
        }
    }

    match transaction.commit().await {
        Ok(_) => HttpResponse::Ok().body(""),
        Err(e) => {
            log!(Level::Error, "Transaction failed to commit");
            HttpResponse::InternalServerError().body(e.to_string())
        }
    }
}
