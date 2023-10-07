use actix_web::{delete, get, post, put, web::{Data, Json}, HttpResponse, Responder};
use log::{log, Level};
use sqlx::{query, query_as};

use crate::{
    api::{log_query, log_query_as, open_transaction},
    app::AppState,
    schema::{api::*, db::AttendanceStatus},
};

#[post("/attendance/house")]
pub async fn submit_hm_attendance(state: Data<AppState>, body: Json<HouseAttendance>) -> impl Responder {
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

    let frosh_id = vec![id; body.frosh.len()];
    let member_id = vec![id; body.members.len()];
    let frosh_names: Vec<i32> = body.frosh.iter().map(|a| a.name).collect();
    let frosh_statuses: Vec<AttendanceStatus> = body.frosh.iter().map(|a| a.att_status).collect();

    match log_query(
        query!("INSERT INTO freshman_hm_attendance (fid, meeting_id, attendance_status) SELECT fid, meeting_id, attendance_status as \"attendance_status: AttendanceStatus\" FROM UNNEST($1::int4[], $2::int4[], $3::attendance_enum[]) as a(fid, meeting_id, attendance_status)", frosh_names.as_slice(), frosh_id.as_slice(), frosh_statuses.as_slice() as &[AttendanceStatus])
        .execute(&state.db).await.map(|_| ()), Some(transaction)).await {
        Ok(tx) => transaction = tx.unwrap(),
        Err(res) => return res,
    }

    // Commit transaction
    match transaction.commit().await {
        Ok(_) => HttpResponse::Ok().body(""),
        Err(e) => {
            log!(Level::Error, "Transaction failed to commit");
            HttpResponse::InternalServerError().body(e.to_string())
        }
    }
}
