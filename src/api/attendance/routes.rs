use crate::api::{log_query, log_query_as, open_transaction};
use crate::app::AppState;
use crate::schema::api::*;
// use crate::schema::db::*;
use actix_web::{
    delete, get, post, put,
    web::{Data, Json, Path},
    HttpResponse, Responder,
};
use log::{log, Level};
use sqlx::{query, query_as};

#[post("/seminar")]
pub async fn submit_seminar_attendance(
    state: Data<AppState>,
    body: Json<MeetingAttendance>,
) -> impl Responder {
    log!(Level::Info, "POST /attendance/seminar");
    let mut transaction = match open_transaction(&state.db).await {
        Ok(t) => t,
        Err(res) => return res,
    };
    log!(Level::Trace, "Acquired transaction");

    let id: i32;

    // Add new technical seminar
    match log_query_as(
        query_as!(
            ID,
            "INSERT INTO technical_seminars (name, timestamp, active, approved)
                VALUES ($1, $2, $3, $4) RETURNING id",
            body.name,
            body.date,
            true,
            false
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
    log!(Level::Debug, "Inserted meeting into db. ID={}", id);

    let frosh_id = vec![id; body.frosh.len()];
    let member_id = vec![id; body.members.len()];

    // Add frosh, seminar relation
    match log_query(
        query!(
            "INSERT INTO freshman_seminar_attendance (fid, seminar_id) SELECT fid, seminar_id
                FROM UNNEST($1::int4[], $2::int4[]) AS a(fid, seminar_id)",
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
        Ok(tx) => {
            transaction = tx.unwrap();
        }
        Err(res) => return res,
    }

    // Add member, seminar relation
    match log_query(
        query!(
            "INSERT INTO member_seminar_attendance (uid, seminar_id) SELECT uid, seminar_id
                FROM UNNEST($1::TEXT[], $2::int4[]) AS a(uid, seminar_id)",
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
        Ok(tx) => {
            transaction = tx.unwrap();
        }
        Err(res) => return res,
    }

    log!(Level::Trace, "Finished adding new seminar attendance");
    // Commit transaction
    match transaction.commit().await {
        Ok(_) => HttpResponse::Ok().body(""),
        Err(e) => {
            log!(Level::Error, "Transaction failed to commit");
            HttpResponse::InternalServerError().body(e.to_string())
        }
    }
}

#[get("/seminar/{user}")]
pub async fn get_seminars_by_user(path: Path<(String,)>, state: Data<AppState>) -> impl Responder {
    let (user,) = path.into_inner();
    log!(Level::Info, "GET /attendance/seminar/{}", user);
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
                Seminar,
                "SELECT ts.name, ts.\"timestamp\", ARRAY[]::varchar[] AS members, ARRAY[]::integer[] AS frosh, ts.approved
                    FROM technical_seminars ts
                    LEFT JOIN freshman_seminar_attendance fsa ON
                    fsa.seminar_id = ts.id
                    WHERE ts.approved
                    AND timestamp > $1::timestamp
                    AND fsa.fid = $2::int4",
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
                Seminar,
                "SELECT ts.name, ts.\"timestamp\", ARRAY[]::varchar[] AS members, ARRAY[]::integer[] AS frosh, ts.approved
                    FROM technical_seminars ts
                    LEFT JOIN member_seminar_attendance msa ON
                    msa.seminar_id = ts.id
                    WHERE ts.approved
                    AND timestamp > $1::timestamp
                    AND msa.uid = $2",
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

#[get("/seminar")]
pub async fn get_seminars(state: Data<AppState>) -> impl Responder {
    log!(Level::Info, "GET /attendance/seminar");
    match query_as!(
        Seminar,
        "SELECT member_seminars.name, member_seminars.timestamp, member_seminars.members, array_agg(fsa.fid) AS frosh, member_seminars.approved
            FROM(SELECT ts.id, ts.name, ts.timestamp, array_agg(msa.uid) AS members, ts.approved
            FROM technical_seminars ts
            INNER JOIN member_seminar_attendance msa ON
            msa.seminar_id = ts.id
            WHERE timestamp > $1::timestamp
            GROUP BY ts.id, ts.name, ts.\"timestamp\", ts.approved) AS member_seminars
            INNER JOIN freshman_seminar_attendance fsa ON
            fsa.seminar_id = member_seminars.id
            GROUP BY member_seminars.id, member_seminars.name, member_seminars.timestamp, member_seminars.members, member_seminars.approved",
        &state.year_start
    )
    .fetch_all(&state.db)
    .await
    {
        Ok(seminars) => HttpResponse::Ok().json(seminars),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[delete("/seminar/{id}")]
pub async fn delete_seminar(path: Path<(String,)>, state: Data<AppState>) -> impl Responder {
    let (id,) = path.into_inner();
    log!(Level::Info, "DELETE /attedance/seminar/{id}");
    let id = match id.parse::<i32>() {
        Ok(id) => id,
        Err(_e) => {
            log!(Level::Warn, "Invalid id");
            return HttpResponse::BadRequest().body("Invalid id");
        }
    };
    let mut transaction = match open_transaction(&state.db).await {
        Ok(t) => t,
        Err(res) => return res,
    };
    log!(Level::Trace, "Acquired transaction");
    match log_query(
        query!(
            "DELETE FROM freshman_seminar_attendance WHERE seminar_id = $1",
            id
        )
        .execute(&state.db)
        .await
        .map(|_| ()),
        Some(transaction),
    )
    .await
    {
        Ok(tx) => {
            transaction = tx.unwrap();
        }
        Err(res) => return res,
    }
    match log_query(
        query!(
            "DELETE FROM member_seminar_attendance WHERE seminar_id = $1",
            id
        )
        .execute(&state.db)
        .await
        .map(|_| ()),
        Some(transaction),
    )
    .await
    {
        Ok(tx) => {
            transaction = tx.unwrap();
        }
        Err(res) => return res,
    }
    match log_query(
        query!("DELETE FROM technical_seminars WHERE id = $1", id)
            .execute(&state.db)
            .await
            .map(|_| ()),
        Some(transaction),
    )
    .await
    {
        Ok(tx) => {
            transaction = tx.unwrap();
        }
        Err(res) => return res,
    }

    log!(Level::Trace, "Finished deleting seminar");
    match transaction.commit().await {
        Ok(_) => HttpResponse::Ok().body(""),
        Err(e) => {
            log!(Level::Error, "Transaction failed to commit");
            HttpResponse::InternalServerError().body(e.to_string())
        }
    }
}

#[put("/seminar/{id}")]
pub async fn edit_seminar_attendance(
    path: Path<(String,)>,
    state: Data<AppState>,
    body: Json<MeetingAttendance>,
) -> impl Responder {
    let (id,) = path.into_inner();
    let id = match id.parse::<i32>() {
        Ok(id) => id,
        Err(_e) => {
            log!(Level::Warn, "Invalid id");
            return HttpResponse::BadRequest().body("Invalid id");
        }
    };
    log!(Level::Info, "PUT /attendance/seminar/{id}");
    let mut transaction = match open_transaction(&state.db).await {
        Ok(t) => t,
        Err(res) => return res,
    };
    log!(Level::Trace, "Acquired transaction");

    match log_query(
        query!(
            "DELETE FROM freshman_seminar_attendance WHERE seminar_id = $1",
            id
        )
        .execute(&state.db)
        .await
        .map(|_| ()),
        Some(transaction),
    )
    .await
    {
        Ok(tx) => transaction = tx.unwrap(),
        Err(res) => return res,
    }

    match log_query(
        query!(
            "DELETE FROM member_seminar_attendance WHERE seminar_id = $1",
            id
        )
        .execute(&state.db)
        .await
        .map(|_| ()),
        Some(transaction),
    )
    .await
    {
        Ok(tx) => transaction = tx.unwrap(),
        Err(res) => return res,
    }

    log!(Level::Trace, "finished deleting existing attendance");

    let frosh_id = vec![id; body.frosh.len()];
    let member_id = vec![id; body.members.len()];

    // Add frosh, seminar relation
    match log_query(
        query!(
            "INSERT INTO freshman_seminar_attendance (fid, seminar_id) SELECT fid, seminar_id
                FROM UNNEST($1::int4[], $2::int4[]) AS a(fid, seminar_id)",
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
        Ok(tx) => {
            transaction = tx.unwrap();
        }
        Err(res) => return res,
    }

    // Add member, seminar relation
    match log_query(
        query!(
            "INSERT INTO member_seminar_attendance (uid, seminar_id) SELECT uid, seminar_id
                FROM UNNEST($1::TEXT[], $2::int4[]) AS a(uid, seminar_id)",
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
        Ok(tx) => {
            transaction = tx.unwrap();
        }
        Err(res) => return res,
    }

    log!(Level::Trace, "Finished adding new seminar attendance");
    // Commit transaction
    match transaction.commit().await {
        Ok(_) => HttpResponse::Ok().body(""),
        Err(e) => {
            log!(Level::Error, "Transaction failed to commit");
            HttpResponse::InternalServerError().body(e.to_string())
        }
    }
}
