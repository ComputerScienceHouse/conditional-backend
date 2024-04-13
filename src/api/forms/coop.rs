use actix_web::{
    get, post,
    web::{Data, Json},
    HttpResponse, Responder,
};
use chrono::{Datelike, Utc};
use sqlx::{query, query_as, Connection};

use crate::{
    api::lib::UserError,
    app::AppState,
    auth_service::{CSHAuth, UserInfo},
    schema::{api::CoopSubmission, db::SemesterEnum},
};

#[utoipa::path(
    context_path = "/api/forms",
    tag = "Forms",
    responses(
        (status = 200, description = "Get a user's coop form", body = Option<CoopSubmission>),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    ),
    security(
        ("csh" = [])
    )
)]
#[get("/coop", wrap = "CSHAuth::member_only()")]
pub async fn get_coop_form(
    state: Data<AppState>,
    user: UserInfo,
) -> Result<impl Responder, UserError> {
    let now = Utc::now();
    let form = query_as!(
        CoopSubmission,
        r#"select uid, year, semester as "semester: SemesterEnum" from 
        coop where year >= $1::int4 and uid = $2::int4"#,
        if now.month() > 5 {
            now.year()
        } else {
            now.year() - 1
        },
        user.get_uid(&state.db).await?
    )
    .fetch_optional(&state.db)
    .await?;
    Ok(HttpResponse::Ok().json(form))
}

#[utoipa::path(
    context_path = "/api/forms",
    tag = "Forms",
    responses(
        (status = 200, description = "Get all coop forms", body = Vec<CoopSubmission>),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    ),
    security(
        ("csh" = ["eboard"])
    )
)]
#[get("/coops", wrap = "CSHAuth::evals_only()")]
pub async fn get_coop_forms(state: Data<AppState>) -> Result<impl Responder, UserError> {
    let now = Utc::now();
    let form = query_as!(
        CoopSubmission,
        r#"select uid, year, semester as "semester: SemesterEnum"
        from coop where year >= $1::int4"#,
        if now.month() > 5 {
            now.year()
        } else {
            now.year() - 1
        },
    )
    .fetch_all(&state.db)
    .await?;
    Ok(HttpResponse::Ok().json(form))
}

#[utoipa::path(
    context_path = "/api/forms",
    tag = "Forms",
    request_body = CoopSubmission,
    responses(
        (status = 200, description = "Submit a coop form"),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    ),
    security(
        ("csh" = [])
    )
)]
#[post("/coop", wrap = "CSHAuth::member_only()")]
pub async fn submit_coop_form(
    state: Data<AppState>,
    user: UserInfo,
    body: Json<CoopSubmission>,
) -> Result<impl Responder, UserError> {
    let now = Utc::now();
    let mut conn = state.db.acquire().await?;
    conn.transaction(|txn| {
        Box::pin(async move {
            query!(
                r#"insert into coop(uid, year, semester)
                values($1::int4, $2::int4, $3::semester_enum)
                on conflict do nothing returning uid"#,
                user.get_uid(&state.db).await?,
                if now.month() > 5 {
                    now.year()
                } else {
                    now.year() - 1
                },
                body.semester as SemesterEnum
            )
            .fetch_optional(&mut **txn)
            .await
        })
    })
    .await?;
    Ok(HttpResponse::Ok().finish())
}
