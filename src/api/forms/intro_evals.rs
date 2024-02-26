use crate::api::lib::{open_transaction, UserError};
use crate::app::AppState;
use crate::auth::{CSHAuth, UserInfo};
use crate::schema::api::IntroForm;
use crate::schema::db::EvalStatusEnum;
use crate::schema::{api, db};
use actix_web::{
    get, post, put,
    web::{Data, Json},
    HttpResponse, Responder,
};
use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, Pool, Postgres};
use utoipa::ToSchema;

#[utoipa::path(
    context_path = "/api/forms",
    tag = "Forms",
    responses(
        (status = 200, description = "Get a user's intro form", body = Option<api::IntroForm>),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    ),
    security(
        ("csh" = []),
        ("intro" = []),
    )
)]
#[get("/intro", wrap = "CSHAuth::member_and_intro()")]
async fn get_intro_form(
    state: Data<AppState>,
    user: UserInfo,
) -> Result<impl Responder, UserError> {
    let intro_form = query_as!(
        IntroForm,
        r#"
        SELECT
            ied.social_events,
            ied.other_comments
        FROM intro_eval_data ied 
        LEFT JOIN "user" u
        	ON u.id = ied.uid
        WHERE (u.ipa_unique_id = $2::varchar OR u.intro_id = $2::varchar) AND eval_block_id = $1::int4
        "#,
        state.eval_block_id,
        user.get_uuid()
    )
    .fetch_optional(&state.db)
    .await?;
    Ok(HttpResponse::Ok().json(intro_form))
}

/// Get all intro forms
///
/// Accessible by: Evaluations
#[utoipa::path(
    context_path = "/api/forms",
    tag = "Forms",
    responses(
        (status = 200, description = "Get all intro forms", body = Vec<api::IntroForm>),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    ),
    security(
        ("csh" = []),
    )
)]
#[get("/intros", wrap = "CSHAuth::evals_only()")]
pub async fn get_all_intro_forms(
    state: Data<AppState>,
    user: UserInfo,
) -> Result<impl Responder, UserError> {
    let forms = query_as!(
        IntroForm,
        r#"
        SELECT
            ied.social_events,
            ied.other_comments
        FROM intro_eval_data ied 
        LEFT JOIN "user" u
        	ON u.id = ied.uid
        WHERE (u.ipa_unique_id = $2::varchar OR u.intro_id = $2::varchar) AND eval_block_id = $1::int4
        "#,
        state.eval_block_id,
        user.get_uuid()
    )
    .fetch_all(&state.db)
    .await?;
    Ok(HttpResponse::Ok().json(forms))
}

/// Submit an intro form
///
/// Accessible by: Intro members
#[utoipa::path(
    context_path = "/api/forms",
    tag = "Forms",
    request_body = IntroFormSubmission,
    responses(
        (status = 200, description = "Sucessfully submitted intro form"),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    ),
    security(
        ("intro" = []),
    )
)]
#[post("/intro/self", wrap = "CSHAuth::intro_only()")]
pub async fn submit_intro_form(
    state: Data<AppState>,
    user: UserInfo,
    body: Json<IntroForm>,
) -> Result<impl Responder, UserError> {
    let mut transaction = open_transaction(&state.db).await?;
    query!(
        r#"insert into intro_eval_data(uid, eval_block_id, social_events, other_comments, status)
        values($1::int4,$2::int4,$3::varchar,$4::varchar,$5::eval_status_enum)
        on conflict on constraint intro_eval_data_pkey do update
        set social_events = $3::varchar, other_comments = $4::varchar
        where intro_eval_data.uid = $1::int4 and intro_eval_data.eval_block_id = $2::int4"#,
        user.get_uid(&state.db).await?,
        state.eval_block_id,
        body.social_events,
        body.other_comments,
        EvalStatusEnum::Pending as EvalStatusEnum,
    )
    .execute(&mut *transaction)
    .await?;
    Ok(HttpResponse::Ok().finish())
}
