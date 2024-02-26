use crate::{
    api::{
        evals::routes::get_intro_member_evals_helper,
        lib::{open_transaction, UserError},
    },
    app::AppState,
    auth::{CSHAuth, UserInfo},
    schema::{
        api::*,
        db::{self, EvalStatusEnum},
    },
};
use actix_web::{
    get, post, put,
    web::{Data, Json},
    HttpResponse, Responder,
};
use sqlx::{query, query_as, query_file_as, Postgres, Transaction};

type PacketNonsense = (
    (Vec<String>, Vec<i32>),
    ((Vec<i64>, Vec<i64>), (Vec<i64>, Vec<i64>)),
);

#[utoipa::path(
    context_path="/api/batch",
    tag = "Batch",
    responses(
        (status = 200, description = "Get all batches", body = Vec<Batch>),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/", wrap = "CSHAuth::enabled()")]
async fn get_all_batches(state: Data<AppState>) -> Result<impl Responder, UserError> {
    let intros: Vec<IntroStatus> = match get_intro_member_evals_helper(&state).await {
        Ok(intros) => intros,
        Err(e) => return Err(e),
    };
    let ((name, uid), ((seminars, directorships), (missed_hms, packet))): PacketNonsense = intros
        .into_iter()
        .filter(|is| {
            is.seminars.is_some()
                && is.directorships.is_some()
                && is.missed_hms.is_some()
                && is.signatures.is_some()
                && is.max_signatures.is_some()
        })
        .map(|is| {
            (
                (is.name, is.uid),
                (
                    (is.seminars.unwrap(), is.directorships.unwrap()),
                    (
                        is.missed_hms.unwrap(),
                        100 * is.signatures.unwrap() / is.max_signatures.unwrap(),
                    ),
                ),
            )
        })
        .unzip();
    // I'm so sorry for anyone who needs to touch this ever
    //
    // I did, it was a terrible experience, and I cannot in good conscience
    // recommend it to anyone
    Ok(HttpResponse::Ok().json(
        query_file_as!(
            Batch,
            "src/queries/get_all_batches.sql",
            &name,
            &uid,
            &seminars,
            &directorships,
            &missed_hms,
            &packet,
        )
        .fetch_all(&state.db)
        .await?,
    ))
}

async fn get_one_batch(state: &Data<AppState>, id: i32) -> Result<Batch, UserError> {
    let intros: Vec<IntroStatus> = match get_intro_member_evals_helper(state).await {
        Ok(intros) => intros,
        Err(e) => return Err(e),
    };
    let ((name, uid), ((seminars, directorships), (missed_hms, packet))): PacketNonsense = intros
        .into_iter()
        .filter(|is| {
            is.seminars.is_some()
                && is.directorships.is_some()
                && is.missed_hms.is_some()
                && is.signatures.is_some()
                && is.max_signatures.is_some()
        })
        .map(|is| {
            (
                (is.name, is.uid),
                (
                    (is.seminars.unwrap(), is.directorships.unwrap()),
                    (
                        is.missed_hms.unwrap(),
                        100 * is.signatures.unwrap() / is.max_signatures.unwrap(),
                    ),
                ),
            )
        })
        .unzip();
    Ok(query_file_as!(
        Batch,
        "src/queries/get_one_batch.sql",
        &name,
        &uid,
        &seminars,
        &directorships,
        &missed_hms,
        &packet,
        id,
    )
    .fetch_one(&state.db)
    .await?)
}

#[utoipa::path(
    context_path = "/api/batch",
    tag = "Batch",
    request_body = BatchSubmission,
    responses(
        (status = 200, description = "Create a batch"),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[post("/", wrap = "CSHAuth::enabled()")]
pub async fn create_batch(
    state: Data<AppState>,
    user: UserInfo,
    body: Json<BatchSubmission>,
) -> Result<impl Responder, UserError> {
    let mut transaction = open_transaction(&state.db).await?;

    // create batch
    let id = query_as!(
        db::ID,
        "INSERT INTO batch(name, creator, approved) VALUES ($1::varchar, $2::int4, false) RETURNING id",
        body.name,
        user.get_uid(&state.db).await?
    )
    .fetch_one(&mut *transaction)
    .await?;

    // add criteria
    let values = body.conditions.iter().map(|a| a.value).collect::<Vec<_>>();
    let criteria = body
        .conditions
        .iter()
        .map(|a| a.criterion)
        .collect::<Vec<_>>();
    let comparisons = body
        .conditions
        .iter()
        .map(|a| a.comparison)
        .collect::<Vec<_>>();
    let batch_ids = vec![id.id; values.len()];

    query!(
        "INSERT INTO batch_condition(value, criterion, comparison, batch_id) SELECT value as \
         \"value!\", criterion AS \"criterion!: BatchCriterion\", comparison AS \
         \"comparison!:_\", batch_id as \"batch_id!\" FROM UNNEST($1::int4[], \
         $2::batch_criterion_enum[], $3::batch_comparison_enum[], $4::int4[]) as a(value, \
         criterion, comparison, batch_id)",
        values.as_slice(),
        criteria.as_slice() as _,
        comparisons.as_slice() as _,
        batch_ids.as_slice()
    )
    .execute(&mut *transaction)
    .await?;

    // add users
    let uids = &body.users;
    let batch_ids = vec![id.id; uids.len()];

    query!(
        "INSERT INTO batch_user(uid, batch_id) SELECT fid, batch_id FROM UNNEST($1::int4[], \
         $2::int4[]) as a(fid, batch_id)",
        uids.as_slice(),
        batch_ids.as_slice()
    )
    .execute(&mut *transaction)
    .await?;

    // Commit transaction
    transaction.commit().await?;
    Ok(HttpResponse::Ok().finish())
}

#[utoipa::path(
    context_path = "/api/batch",
    tag = "Batch",
    request_body = BatchPull,
    responses(
        (status = 200, description = "Pull user"),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[post("/pull", wrap = "CSHAuth::evals_only()")]
pub async fn pull_user(
    state: Data<AppState>,
    body: Json<BatchPull>,
) -> Result<impl Responder, UserError> {
    let mut transaction = open_transaction(&state.db).await?;

    query!(
        "update batch_pull set approved = true where uid = $1::int4 and puller = $2::int4",
        body.uid,
        body.puller
    )
    .execute(&mut *transaction)
    .await?;

    // Commit transaction
    transaction.commit().await?;
    Ok(HttpResponse::Ok().finish())
}

#[utoipa::path(
    context_path = "/api/batch",
    tag = "Batch",
    request_body = BatchPull,
    responses(
        (status = 200, description = "Request that a user be pulled"),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[post("/pr", wrap = "CSHAuth::enabled()")]
pub async fn submit_batch_pr(
    state: Data<AppState>,
    user: UserInfo,
    body: Json<BatchPull>,
) -> Result<impl Responder, UserError> {
    let mut transaction = open_transaction(&state.db).await?;

    query!(
        "insert into batch_pull(uid, puller, reason, approved) values($1::int4, $2::int4, $3::varchar, false) on \
         conflict on constraint batch_pull_pkey do update set reason = $3::varchar",
        body.uid,
        user.get_uid(&state.db).await?,
        body.reason,
    )
    .execute(&mut *transaction)
    .await?;

    // Commit transaction
    transaction.commit().await?;
    Ok(HttpResponse::Ok().finish())
}

#[utoipa::path(
    context_path = "/api/batch",
    tag = "Batch",
    responses(
        (status = 200, description = "Get all pull requests", body = Vec<BatchPull>),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[get("/pr", wrap = "CSHAuth::evals_only()")]
pub async fn get_pull_requests(state: Data<AppState>) -> Result<impl Responder, UserError> {
    let prs = query_as!(BatchPull, "select uid, reason, puller from batch_pull")
        .fetch_all(&state.db)
        .await?;

    Ok(HttpResponse::Ok().json(prs))
}

async fn execute_batch_action<'a>(
    batch_id: i32,
    state: &Data<AppState>,
    transaction: Transaction<'a, Postgres>,
    action: EvalStatusEnum,
) -> Result<Transaction<'a, Postgres>, UserError> {
    let batch = get_one_batch(state, batch_id).await?;

    query!(
        "
        UPDATE intro_eval_data
        SET status = $2::eval_status_enum
        FROM UNNEST($1::int4[]) as u
        WHERE uid = u
        ",
        &batch.members,
        action as EvalStatusEnum,
    )
    .execute(&state.db)
    .await?;

    Ok(transaction)
}

#[utoipa::path(
    context_path="/api/batch",
    tag = "Batch",
    request_body = i32,
    responses(
        (status = 200, description = "Pass every user in the batch"),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[put("/pass", wrap = "CSHAuth::evals_only()")]
pub async fn pass_batch(
    state: Data<AppState>,
    body: Json<i32>,
) -> Result<impl Responder, UserError> {
    let mut transaction = open_transaction(&state.db).await?;

    transaction = execute_batch_action(*body, &state, transaction, EvalStatusEnum::Passed).await?;

    // Commit transaction
    transaction.commit().await?;

    Ok(HttpResponse::Ok().finish())
}

#[utoipa::path(
    context_path="/api/batch",
    tag = "Batch",
    request_body = i32,
    responses(
        (status = 200, description = "Fail every user in the batch"),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    )
)]
#[put("/fail", wrap = "CSHAuth::evals_only()")]
pub async fn fail_batch(
    state: Data<AppState>,
    body: Json<i32>,
) -> Result<impl Responder, UserError> {
    let mut transaction = open_transaction(&state.db).await?;

    transaction = execute_batch_action(*body, &state, transaction, EvalStatusEnum::Failed).await?;

    // Commit transaction
    transaction.commit().await?;

    Ok(HttpResponse::Ok().finish())
}
