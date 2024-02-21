use crate::api::lib::{open_transaction, UserError};
use crate::app::AppState;
use crate::auth::{CSHAuth, UserInfo};
use crate::schema::{api, db};
use actix_web::{
    get, post, put,
    web::{Data, Json},
    HttpResponse, Responder,
};
use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, Pool, Postgres};
use utoipa::ToSchema;

/// Get the latest intro form related to the user during the current evaluation
/// block
async fn get_intro_form(
    db: &Pool<Postgres>,
    user: &UserInfo,
    evals_block_id: i32,
) -> Result<Option<(i32, api::IntroForm)>, UserError> {
    #[derive(Serialize, Deserialize, Clone, Debug)]
    struct IntroForm {
        id: i32,
        social_events: String,
        other_comments: String,
        status: db::EvalStatusEnum,
    }
    if let Some(intro_form) = query_as!(
        IntroForm,
        r#"
        SELECT
            ied.id,
            ied.social_events,
            ied.other_comments,
            ied.status as "status: db::EvalStatusEnum"
        FROM intro_eval_data ied 
        LEFT JOIN "user" u
        	ON u.id = ied.uid
        WHERE (u.ipa_unique_id = $2::varchar OR u.intro_id = $2::varchar) AND
            ((
                eval_block_id = $1::int4
                AND ied.status != 'Passed'
            ) OR (
                eval_block_id <= $1::int4
                AND ied.status = 'Passed'
            ))
        ORDER BY
            ied.eval_block_id DESC, -- to grab the latest, for members who have passed
            ied.id DESC -- for legacy data; takes the latest submission when users could submit more than once
        LIMIT 1"#,
        evals_block_id,
        user.get_uuid()
    )
    .fetch_optional(db)
        .await? {
            Ok(Some((intro_form.id, api::IntroForm {
                social_events: intro_form.social_events,
                other_comments: intro_form.other_comments,
                status: intro_form.status,
            })))
        } else {
            Ok(None)
        }
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct IntroFormSubmission {
    pub social_events: String,
    pub comments: Option<String>,
}

/// Get an intro form
///
/// Accessible by: CSH and intro members
#[utoipa::path(
    context_path = "/api/forms",
    tag = "Forms",
    responses(
        (status = 200, description = "Get a user's intro form", body = Option<api::IntroForm>),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "No intro form"),
        (status = 500, description = "Internal Server Error"),
    ),
    security(
        ("csh" = []),
        ("intro" = []),
    )
)]
#[get("/intro/self", wrap = "CSHAuth::member_and_intro()")]
pub async fn get_user_intro_form(
    state: Data<AppState>,
    user: UserInfo,
) -> Result<impl Responder, UserError> {
    if let Some((_id, intro_form)) = get_intro_form(&state.db, &user, state.eval_block_id).await? {
        Ok(HttpResponse::Ok().json(intro_form))
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}

/// Get all intro forms
///
/// Accessible by: Evaluations
#[utoipa::path(
    context_path = "/api/forms",
    tag = "Forms",
    responses(
        // (status = 200, description = "Get a user's intro form", body = Option<api::IntroForm>),
        // (status = 401, description = "Unauthorized"),
        // (status = 404, description = "No intro form"),
        // (status = 500, description = "Internal Server Error"),
        (status = 501, description = "Not implemented"),
    ),
    security(
        ("csh" = []),
    )
)]
#[get("/intro", wrap = "CSHAuth::evals_only()")]
pub async fn get_all_intro_forms(_state: Data<AppState>) -> Result<impl Responder, UserError> {
    // Ok(query_as!(
    //     IntroForm,
    //     r#"
    //     "#,
    //     state.eval_block_id
    // )
    // .fetch_all(&state.db)
    // .await?)
    Ok(HttpResponse::NotImplemented().finish())
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
        (status = 409, description = "User cannot submit the form again.", body = String),
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
    body: Json<IntroFormSubmission>,
) -> Result<impl Responder, UserError> {
    let mut transaction = open_transaction(&state.db).await?;
    if let Some((_id, intro_form)) = get_intro_form(&state.db, &user, state.eval_block_id).await? {
        match intro_form.status {
            db::EvalStatusEnum::Pending => Ok(HttpResponse::Conflict()
                .body("User is pending evaulation. Please use PUT to update your intro form.")),
            db::EvalStatusEnum::Passed => {
                Ok(HttpResponse::Conflict()
                    .body("User has already passed, and cannot submit again."))
            }
            db::EvalStatusEnum::Failed => Ok(HttpResponse::Conflict()
                .body("User has already failed this evaluations block, and cannot submit again.")),
        }
    } else {
        let IntroFormSubmission {
            social_events,
            comments,
        } = body.0;
        query!(
            r#"
            INSERT INTO intro_eval_data (uid, eval_block_id, social_events, other_comments, status)
            VALUES ($1::int4, $2::int4, $3::varchar, $4::varchar, $5::eval_status_enum)
            "#,
            user.get_uid(&state.db).await?,
            state.eval_block_id,
            social_events,
            comments.unwrap_or("".to_string()),
            db::EvalStatusEnum::Pending as db::EvalStatusEnum
        )
        .execute(&mut *transaction)
        .await?;
        transaction.rollback().await?;
        Ok(HttpResponse::Ok().finish())
    }
}

/// Update an intro form
///
/// Accessible by: Intro members
#[utoipa::path(
    context_path = "/api/forms",
    tag = "Forms",
    request_body = IntroFormSubmission,
    responses(
        (status = 200, description = "Sucessfully updated intro form"),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    ),
    security(
        ("intro" = []),
    )
)]
#[put("/intro/self", wrap = "CSHAuth::intro_only()")]
pub async fn update_intro_form(
    state: Data<AppState>,
    user: UserInfo,
    body: Json<IntroFormSubmission>,
) -> Result<impl Responder, UserError> {
    let mut transaction = open_transaction(&state.db).await?;
    if let Some(_) = get_intro_form(&state.db, &user, state.eval_block_id).await? {
        Ok(HttpResponse::Conflict().finish())
    } else {
        let IntroFormSubmission {
            social_events,
            comments,
        } = body.0;
        query!(
            r#"
            UPDATE intro_eval_data
            SET
                social_events = $1::varchar,
                other_comments = $2::varchar
            WHERE uid = $3::int4 AND eval_block_id = $4::int4
            "#,
            social_events,
            comments.unwrap_or("".to_string()),
            user.get_uid(&state.db).await?,
            state.eval_block_id,
        )
        .execute(&mut *transaction)
        .await?;
        Ok(HttpResponse::NotImplemented().finish())
    }
}
