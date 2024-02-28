use crate::api::lib::UserError;
use crate::app::AppState;
use crate::auth::{CSHAuth, UserInfo};
use crate::schema::api::MajorProjectSubmission;
use crate::schema::db::MajorProjectStatusEnum;
use crate::schema::{api, db};
use actix_web::{
    get, post,
    web::{Data, Json},
    HttpResponse, Responder,
};
use chrono::{Datelike, NaiveDate, Utc};
use sqlx::{query, query_as, Connection};

#[utoipa::path(
    context_path = "/api/forms",
    tag = "Forms",
    responses(
        (status = 200, description = "Get a user's major project form", body = Vec<api::MajorProjectSubmission>),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "No intro form"),
        (status = 500, description = "Internal Server Error"),
    ),
    security(
        ("csh" = []),
        ("intro" = []),
    )
)]
#[get("/major", wrap = "CSHAuth::member_and_intro()")]
async fn get_user_major_projects(
    state: Data<AppState>,
    user: UserInfo,
) -> Result<impl Responder, UserError> {
    let now = Utc::now();
    let projects = query_as!(
        api::MajorProjectSubmission,
        r#"
        SELECT
            mp.id,
            u.id uid,
            mp.name,
            mp.description,
            mp.status as "status: db::MajorProjectStatusEnum"
        FROM major_project mp 
        LEFT JOIN "user" u
        	ON u.id = mp.uid
        WHERE (u.ipa_unique_id = $2::varchar OR u.intro_id = $2::varchar) AND date > $1
        order by date"#,
        if now.month() > 5 {
            NaiveDate::from_ymd_opt(now.year(), 6, 1).unwrap()
        } else {
            NaiveDate::from_ymd_opt(now.year() - 1, 6, 1).unwrap()
        },
        user.get_uuid()
    )
    .fetch_all(&state.db)
    .await?;
    Ok(HttpResponse::Ok().json(projects))
}

#[utoipa::path(
    context_path = "/api/forms",
    tag = "Forms",
    responses(
        (status = 200, description = "Get all major project forms", body = Vec<api::MajorProjectSubmission>),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "No intro form"),
        (status = 500, description = "Internal Server Error"),
    ),
    security(
        ("csh" = ["eboard"]),
    )
)]
#[get("/majors", wrap = "CSHAuth::evals_only()")]
pub async fn get_all_major_projects(state: Data<AppState>) -> Result<impl Responder, UserError> {
    let now = Utc::now();
    let projects = query_as!(
        api::MajorProjectSubmission,
        r#"
        SELECT
            mp.id,
            u.id uid,
            mp.name,
            mp.description,
            mp.status as "status: db::MajorProjectStatusEnum"
        FROM major_project mp 
        LEFT JOIN "user" u
        	ON u.id = mp.uid
        WHERE date > $1
        order by date"#,
        if now.month() > 5 {
            NaiveDate::from_ymd_opt(now.year(), 6, 1).unwrap()
        } else {
            NaiveDate::from_ymd_opt(now.year() - 1, 6, 1).unwrap()
        }
    )
    .fetch_all(&state.db)
    .await?;
    Ok(HttpResponse::Ok().json(projects))
}

#[utoipa::path(
    context_path = "/api/forms",
    tag = "Forms",
    request_body = MajorProjectSubmission,
    responses(
        (status = 200, description = "Sucessfully submitted major project"),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    ),
    security(
        ("csh" = []),
        ("intro" = []),
    )
)]
#[post("/major", wrap = "CSHAuth::member_and_intro()")]
pub async fn submit_major_project(
    state: Data<AppState>,
    user: UserInfo,
    body: Json<MajorProjectSubmission>,
) -> Result<impl Responder, UserError> {
    let mut conn = state.db.acquire().await?;
    conn.transaction(|txn| {
        Box::pin(async move {
            query!(
                r#"insert into major_project(uid, name, description, date, status) values($1,$2,$3,$4,$5)"#,
                user.get_uid(&state.db).await?,
                body.name,
                body.description,
                Utc::now().date_naive(),
                MajorProjectStatusEnum::Pending as MajorProjectStatusEnum
            )
            .execute(&mut **txn)
            .await
        })
    })
    .await?;
    Ok(HttpResponse::Ok().finish())
}
