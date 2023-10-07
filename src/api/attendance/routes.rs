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

#[post("/attendance/seminar")]
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
        query_as!(ID, "INSERT INTO technical_seminars (name, timestamp, active, approved) VALUES ($1, $2, $3, $4) RETURNING id", body.name, body.date, true, false).fetch_all(&state.db).await, 
        transaction
        ).await {
        Ok((tx, i)) => {
            transaction = tx;
            id = i[0].id;
        },
        Err(res) => return res,
    }
    log!(Level::Debug, "Inserted meeting into db. ID={}", id);

    let frosh_id = vec![id; body.frosh.len()];
    let member_id = vec![id; body.members.len()];

    // Add frosh, seminar relation
    match log_query(
        query!("INSERT INTO freshman_seminar_attendance (fid, seminar_id) SELECT fid, seminar_id FROM UNNEST($1::int4[], $2::int4[]) as a(fid, seminar_id)", body.frosh.as_slice(), frosh_id.as_slice()).fetch_all(&state.db).await.map(|_| ()),
        transaction
        ).await {
        Ok(tx) => {
            transaction = tx;
        },
        Err(res) => return res,
    }

    // Add member, seminar relation
    match log_query(
        query!("INSERT INTO member_seminar_attendance (uid, seminar_id) SELECT uid, seminar_id FROM UNNEST($1::text[], $2::int4[]) as a(uid, seminar_id)", body.members.as_slice(), member_id.as_slice()).fetch_all(&state.db).await.map(|_| ()),
        transaction
        ).await {
        Ok(tx) => {
            transaction = tx;
        },
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

#[get("/attendance/seminar/{user}")]
pub async fn get_seminars_by_user(
    path: Path<(String, String)>,
    state: Data<AppState>,
) -> impl Responder {
    let (user, _) = path.into_inner();
    log!(Level::Info, "GET /attendance/seminar/{}", user);
    let transaction = match open_transaction(&state.db).await {
        Ok(t) => t,
        Err(res) => return res,
    };
    log!(Level::Trace, "Acquired transaction");


    if user.chars().next().unwrap().is_numeric() {
          let user: i32 = match user.parse() {
            Ok(user) => user,
            Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
          };
          match log_query_as(query_as!(
            Seminar,
            "select ts.name, ts.\"timestamp\", array[]::varchar[] as members, array[]::integer[] as frosh from
            technical_seminars ts 
            left join freshman_seminar_attendance fsa on fsa.seminar_id  = ts.id
            where 
                ts.approved
                and timestamp > $1::timestamp
                and fsa.fid = $2::int4",
            &state.year_start, user) 
                .fetch_all(&state.db)
                .await, transaction).await
            {
                Ok((_, seminars)) => {
                    HttpResponse::Ok().json(seminars)
                }
                Err(e) => return e,
            }
        } else {
          match log_query_as(query_as!(
            Seminar,
            "select ts.name, ts.\"timestamp\", array[]::varchar[] as members, array[]::integer[] as frosh from
                technical_seminars ts 
                left join member_seminar_attendance msa on msa.seminar_id  = ts.id
                where 
                    ts.approved
                    and timestamp > $1::timestamp
                    and msa.uid = $2",
            &state.year_start, user) 
                .fetch_all(&state.db)
                .await, transaction).await
            {
                Ok((_, seminars)) => {
                    HttpResponse::Ok().json(seminars)
                }
                Err(e) => return e,
            }
        }
}

#[get("/attendance/seminar")]
pub async fn get_seminars(state: Data<AppState>) -> impl Responder {
    match query_as!(
    Seminar,
    "SELECT member_seminars.name, member_seminars.timestamp, member_seminars.members, array_agg(fsa.fid) as frosh FROM
        (SELECT ts.id, ts.name, ts.timestamp, array_agg(msa.uid) as members FROM technical_seminars ts 
            LEFT JOIN member_seminar_attendance msa on msa.seminar_id = ts.id
            WHERE timestamp > $1::timestamp
            GROUP BY ts.id, ts.name, ts.\"timestamp\") as member_seminars
                LEFT JOIN freshman_seminar_attendance fsa on fsa.seminar_id = member_seminars.id
                GROUP BY member_seminars.id, member_seminars.name, member_seminars.timestamp, member_seminars.members",
    &state.year_start
  )
  .fetch_all(&state.db)
  .await
  {
    Ok(seminars) => HttpResponse::Ok().json(seminars),
    Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
  }
}

/*
#[put("/attendance/seminar/{id}")]
pub async fn put_seminar(state: Data<AppState>, body: Json<String>) -> impl Responder {
    let (id,) = path.into_inner();
    let usernames: Vec<&String> = body.iter();
    let frosh: Vec<u32> = usernames.filter(|a| {
        let c = a.chars().next();
        if c.is_some() {
            c.unwrap().is_numeric()
        }
    }).map(|a| *a.parse()).collect();
    let members = usernames.filter(|a| {
        let c = a.chars().next();
        if c.is_some() {
            !c.unwrap().is_numeric()
        }
    }).collect::<Vec<_>>();
    let seminar_id_vec = vec![id; usernames.len()];
    match query!("DELETE FROM freshman_seminar_attendance WHERE seminar_id = ($1::i32); DELETE FROM member_seminar_attendance WHERE seminar_id = ($2::i32); INSERT INTO freshman_seminar_attendance(fid, seminar_id) SELECT * FROM UNNEST($3::int4[], $4::int4[]); INSERT INTO member_seminar_attendance(uid, seminar_id) SELECT * FROM UNNEST($5::text[], $6::int4[]);", id, id, frosh, seminar_id_vec, members, seminar_id_vec)
        .execute(&state.db)
        .await
        {
            Ok(_) => HttpResponse::Ok(),
            Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
        }
}

#[delete("/attendance/seminar/{id}")]
pub async fn delete_seminar(state: Data<AppState>) -> impl Responder {
    let (id,) = path.into_inner();
    match query!("DELETE FROM freshman_seminar_attendance WHERE seminar_id = ($1::int4);
    DELETE FROM member_seminar_attendance WHERE seminar_id = ($2::int4);
    DELETE FROM technical_seminars WHERE id = ($3::int4);", id, id, id)
        .execute(&state.db)
        .await
    {
        Ok(_) => HttpResponse::Ok(),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

*/

#[delete("/attedance/seminar/{id}")]
pub async fn delete_seminar(path: Path<(String, String)>, state: Data<AppState>) -> impl Responder {
    let (id, _) = path.into_inner();
    log!(Level::Info, "DELETE /attedance/seminar/{id}");
    let id = match id.parse::<i32>() {
        Ok(id) => id,
        Err(e) => {
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
        transaction,
    )
    .await
    {
        Ok(tx) => {
            transaction = tx;
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
        transaction,
    )
    .await
    {
        Ok(tx) => {
            transaction = tx;
        }
        Err(res) => return res,
    }
    match log_query(
        query!("DELETE FROM technical_seminars WHERE id = $1", id)
            .execute(&state.db)
            .await
            .map(|_| ()),
        transaction,
    )
    .await
    {
        Ok(tx) => {
            transaction = tx;
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

/*

// TODO: Joe: committee is used over directorship to maintain parity with db
#[post("/attendance/committee")]
pub async fn submit_committee_attendance(state: Data<AppState>, body: MeetingAttendance) -> impl Responder {
    // TODO: eboard should auto approve
    let id = match query_as!(i32, "INSERT INTO committee_meetings(committee, timestamp, active, approved) OUTPUT INSERTED.id VALUES ($1::varchar(128), $2::timestamp, true, $3::bool", body.name, body.date, false)
        .fetch_one(&state.db)
        .await
    {
        Ok(id) => id,
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };

    let usernames: Vec<&String> = body.iter();
    let frosh: Vec<u32> = usernames.filter(|a| {
        let c = a.chars().next();
        if c.is_some() {
            c.unwrap().is_numeric()
        }
    }).map(|a| *a.parse()).collect();
    let members = usernames.filter(|a| {
        let c = a.chars().next();
        if c.is_some() {
            !c.unwrap().is_numeric()
        }
    }).collect::<Vec<_>>();
    let committee_id_vec = vec![id; usernames.len()];
    match query!("DELETE FROM freshman_committee_attendance WHERE meeting_id = ($1::i32); DELETE FROM member_committee_attendance WHERE meeting_id = ($2::i32); INSERT INTO freshman_committee_attendance(fid, meeting_id) SELECT * FROM UNNEST($3::int4[], $4::int4[]); INSERT INTO member_committee_attendance(uid, meeting_id) SELECT * FROM UNNEST($5::text[], $6::int4[]);", id, id, frosh, committee_id_vec, members, committee_id_vec)
        .execute(&state.db)
        .await
    {
        Ok(_) => HttpResponse::Ok(),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[get("/attendance/committee/{user}")]
pub async fn get_committees_by_user(state: Data<AppState>) -> impl Responder {
    // TODO: authenticate with token
    let (name,) = path.into_inner();
    if name.len() < 1 {
        return HttpResponse::BadRequest().body("No name found".to_string());
    }
    match query_as!(Committee, format!("SELECT * FROM {} WHERE approved = 'true' AND {} = $1 AND committee_id IN (SELECT id FROM committee_meetings WHERE timestamp > ($2::timestamp))", if name.chars().next().is_numeric() { "freshman_committee_attendance" } else { "member_committee_attendance" }, if name.chars().next().is_numeric() { "fid" } else { "uid" }), body.name, &state.year_start)
        .fetch_all(&state.db)
        .await
    {
        Ok(committees) => HttpResponse::Ok().json(committees),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[get("/attendance/committee")]
pub async fn get_committee(state: Data<AppState>) -> impl Responder {
    // TODO: Joe: year_start should be the day the new year button was pressed by Evals, formatted for postgres
    match query_as!(Committee, "SELECT * FROM committee_meetings WHERE timestamp > ($1::timestamp)", &state.year_start)
        .fetch_all(&state.db)
        .await
    {
        Ok(committees) => HttpResponse::Ok().json(committees),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[put("/attendance/committee/{id}")]
pub async fn put_committee(state: Data<AppState>, body: Json<String>) -> impl Responder {
    let (id,) = path.into_inner();
    let usernames: Vec<&String> = body.iter();
    let frosh: Vec<u32> = usernames.filter(|a| {
        let c = a.chars().next();
        if c.is_some() {
            c.unwrap().is_numeric()
        }
    }).map(|a| *a.parse()).collect();
    let members = usernames.filter(|a| {
        let c = a.chars().next();
        if c.is_some() {
            !c.unwrap().is_numeric()
        }
    }).collect::<Vec<_>>();
    let committee_id_vec = vec![id; usernames.len()];
    match query!("DELETE FROM freshman_committee_attendance WHERE meeting_id = ($1::i32); DELETE FROM member_committee_attendance WHERE meeting_id = ($2::i32); INSERT INTO freshman_committee_attendance(fid, meeting_id) SELECT * FROM UNNEST($3::int4[], $4::int4[]); INSERT INTO member_committee_attendance(uid, meeting_id) SELECT * FROM UNNEST($5::text[], $6::int4[]);", id, id, frosh, committee_id_vec, members, committee_id_vec)
        .execute(&state.db)
        .await
    {
        Ok(_) => HttpResponse::Ok(),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[delete("/attendance/committee/{id}")]
pub async fn delete_committee(state: Data<AppState>) -> impl Responder {
    let (id,) = path.into_inner();
    match query!("DELETE FROM freshman_committee_attendance WHERE meeting_id = ($1::int4); DELETE FROM member_committee_attendance WHERE meeting_id = ($2::int4); DELETE FROM committee_meetings WHERE id = ($3::int4);", id, id, id)
        .execute(&state.db)
        .await
    {
        Ok(_) => HttpResponse::Ok(),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[post("attendance/house")]
pub async fn submit_hm_attendance(state: Data<AppState>, body: HouseAttendance) -> impl Responder {
    let id = match query_as!(i32, "INSERT INTO house_meetings(date, active) OUTPUT INSERTED.id VALUES ($1::timestamp, true)", body.date)
        .fetch_one(&state.db)
        .await
    {
        Ok(id) => id,
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    };

    let usernames: Vec<&String> = body.body.iter();
    let frosh: Vec<u32> = usernames.filter(|a| {
        let c = a.name.chars().next();
        if c.is_some() {
            c.unwrap().is_numeric()
        }
    });
    let frosh_names = frosh.map(|a| *a.name.parse()).collect();
    let frosh_att = frosh.map(|a| a.att_status).collect::<Vec<_>>();
    let members = usernames.filter(|a| {
        let c = a.name.chars().next();
        if c.is_some() {
            !c.unwrap().is_numeric()
        }
    });
    let member_names = members.map(|a| a.name).collect::<Vec<_>>();
    let member_att = members.map(|a| a.att_status).collect::<Vec<_>>();
    let committee_id_vec = vec![id; usernames.len()];
    match query!("INSERT INTO freshman_hm_attendance(fid, meeting_id, attendance_status) SELECT * FROM UNNEST($1::int4[], $2::int4[], $3::attendance_enum[]); INSERT INTO member_hm_attendance(uid, meeting_id, attendance_status) SELECT * FROM UNNEST($4::text[], $5::int4[], $6::attendance_enum[]);", frosh_names, committee_id_vec, frosh_att, members_names, committee_id_vec, frosh_att)
        .execute(&state.db)
        .await
    {
        Ok(_) => HttpResponse::Ok(),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[get("/attendance/house/{user}")]
// TODO: confirm that if user is "" the evals-only get request will be called instead
pub async fn get_hm_attendance_by_user(state: Data<AppState>) -> impl Responder {
    let (user,) = path.into_inner();
    let table = if user.chars().next().unwrap().is_numeric() { "freshman_hm_attendance" } else { "member_hm_attendance" };
    match query_as!(IndividualHouseAttendance, format!("SELECT * FROM {} WHERE attendance_status = 'Absent' AND timestamp > $1::timestamp", table), &state.year_start)
        .fetch_all()
        .await
    {
        Ok(hms) => HttpResponse::Ok().json(hms),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

// Evals only for member management
#[get("/attendance/house-evals/{user}")]
pub async fn get_hm_attendance_by_user(state: Data<AppState>) -> impl Responder {
    let (user,) = path.into_inner();
    let table = if user.chars().next().unwrap().is_numeric() { "freshman_hm_attendance" } else { "member_hm_attendance" };
    match query_as!(IndividualHouseAttendance, format!("SELECT * FROM {} WHERE attendance_status != 'Attended' AND timestamp > $1::timestamp", table), &state.year_start)
        .fetch_all()
        .await
    {
        Ok(hms) => HttpResponse::Ok().json(hms),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

// #[get("/attendance/house")]
// where is this used

#[put("/attendance/house/{id}")]
pub async fn update_hm_attendance(state: Data<AppState>, body: IndividualHouseAttendance) -> impl Responder {
    let (id,) = path.into_inner();
    let table = if user.chars().next().unwrap().is_numeric() { "freshman_hm_attendance" } else { "member_hm_attendance" };
    let id_col_name = if user.chars().next().unwrap().is_numeric() { "fid" } else { "uid" };
    match query!(format!("DELETE FROM {} WHERE {} = $1; INSERT INTO {}({}, meeting_id, attendance_status) VALUES ($2, $3, $4);", table, id_col_name, table, id_col_name), body.name, id, body.att_status)
        .execute(&state.db)
        .await
    {
        Ok(_) => HttpResponse::Ok(),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[delete("/attendance/house/{id}")]
pub async fn delete_hm(state: Data<AppState>) -> impl Responder {
    let (id,) = path.into_inner();
    match query!("DELETE FROM freshman_hm_attendance WHERE meeting_id = $1; DELETE FROM member_hm_attendance WHERE meeting_id = $2; DELETE FROM house_meetings WHERE id = #3", id, id, id)
        .execute(&state.db)
        .await
    {
        Ok(_) => HttpResponse::Ok(),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}
*/
