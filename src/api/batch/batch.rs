use actix_web::{delete, get, post, put, web::{Data, Json, Path}, Responder, HttpResponse,};
use log::{log, Level};
use sqlx::{query, query_as};

use crate::{schema::{api::*, db::{BatchComparison, BatchConditionType}}, app::AppState, api::{open_transaction, log_query, log_query_as}};

#[post("/evals/batch/{user}")]
pub async fn create_batch(path: Path<(String,)>, state: Data<AppState>, body: Json<BatchSubmission>) -> impl Responder {
    let (user,) = path.into_inner();
    let body = body.into_inner();
    log!(Level::Info, "POST /evals/batch/{user}");
    let mut transaction = match open_transaction(&state.db).await {
        Ok(t) => t,
        Err(res) => return res,
    };

    // create batch
    let id: i32;
    match log_query_as(query_as!(ID, "INSERT INTO batch(name, uid, approved) VALUES ($1, $2, $3) RETURNING id", body.name, user, false).fetch_all(&state.db).await, Some(transaction)).await {
        Ok((tx, i)) => {
            transaction = tx.unwrap();
            id = i[0].id;
        }
        Err(res) => return res,
    }

    // add conditions
    let values = body.conditions.iter().map(|a| a.value).collect::<Vec<_>>();
    let conditions = body.conditions.iter().map(|a| a.condition).collect::<Vec<_>>();
    let comparisons = body.conditions.iter().map(|a| a.comparison).collect::<Vec<_>>();
    let batch_ids = vec![id; values.len()];
    match log_query(query!("INSERT INTO batch_conditions(value, condition, comparison, batch_id) SELECT value as \"value!\", condition AS \"condition!:_\", comparison AS \"comparison!:_\", batch_id as \"batch_id!\" FROM UNNEST($1::int4[], $2::batch_ctype_enum[], $3::batch_comparison[], $4::int4[]) as a(value, condition, comparison, batch_id)", values.as_slice(), conditions.as_slice() as &[BatchConditionType], comparisons.as_slice() as &[BatchComparison], batch_ids.as_slice()).execute(&state.db).await.map(|_| ()), Some(transaction)).await {
        Ok(tx) => transaction = tx.unwrap(),
        Err(res) => return res,
    }

    // add users
    let fids = body.freshman_users.iter().map(|a| a.fid).collect::<Vec<_>>();
    let batch_ids = vec![id; fids.len()];
    match log_query(query!("INSERT INTO freshman_batch_users(fid, batch_id) SELECT fid, batch_id FROM UNNEST($1::int4[], $2::int4[]) as a(fid, batch_id)", fids.as_slice(), batch_ids.as_slice()).execute(&state.db).await.map(|_| ()), Some(transaction)).await {
        Ok(tx) => transaction = tx.unwrap(),
        Err(res) => return res,
    }

    let uids = body.member_users.iter().map(|a| a.uid.clone()).collect::<Vec<_>>();
    let batch_ids = vec![id; uids.len()];
    match log_query(query!("INSERT INTO member_batch_users(uid, batch_id) SELECT uid, batch_id FROM UNNEST($1::text[], $2::int4[]) as a(uid, batch_id)", uids.as_slice(), batch_ids.as_slice()).execute(&state.db).await.map(|_| ()), Some(transaction)).await {
        Ok(tx) => transaction = tx.unwrap(),
        Err(res) => return res,
    }

    // Commit trnnsaction
    match transaction.commit().await {
        Ok(_) => HttpResponse::Created().finish(),
        Err(e) => {
            log!(Level::Error, "Transaction failed to commit");
            HttpResponse::InternalServerError().body(e.to_string())
        }
    }
}
