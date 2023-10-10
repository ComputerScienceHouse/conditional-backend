use actix_web::{
    delete, get, post, put,
    web::{Data, Json, Path},
    HttpResponse, Responder,
};
use log::{log, Level};
use sqlx::{query, query_as};

use crate::{
    api::{evals::routes::get_intro_member_evals, log_query, log_query_as, open_transaction},
    app::AppState,
    schema::{
        api::*,
        db::{BatchComparison, BatchConditionType},
    },
};

#[post("/batch/{user}")]
pub async fn create_batch(
    path: Path<(String,)>,
    state: Data<AppState>,
    body: Json<BatchSubmission>,
) -> impl Responder {
    let (user,) = path.into_inner();
    let body = body.into_inner();
    log!(Level::Info, "POST /evals/batch/{user}");
    let mut transaction = match open_transaction(&state.db).await {
        Ok(t) => t,
        Err(res) => return res,
    };

    // create batch
    let id: i32;
    match log_query_as(
        query_as!(
            ID,
            "INSERT INTO batch(name, uid, approved) VALUES ($1, $2, $3) RETURNING id",
            body.name,
            user,
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

    // add conditions
    let values = body.conditions.iter().map(|a| a.value).collect::<Vec<_>>();
    let conditions = body
        .conditions
        .iter()
        .map(|a| a.condition)
        .collect::<Vec<_>>();
    let comparisons = body
        .conditions
        .iter()
        .map(|a| a.comparison)
        .collect::<Vec<_>>();
    let batch_ids = vec![id; values.len()];
    match log_query(query!("INSERT INTO batch_conditions(value, condition, comparison, batch_id) SELECT value as \"value!\", condition AS \"condition!:_\", comparison AS \"comparison!:_\", batch_id as \"batch_id!\" FROM UNNEST($1::int4[], $2::batch_ctype_enum[], $3::batch_comparison[], $4::int4[]) as a(value, condition, comparison, batch_id)", values.as_slice(), conditions.as_slice() as &[BatchConditionType], comparisons.as_slice() as &[BatchComparison], batch_ids.as_slice()).execute(&state.db).await.map(|_| ()), Some(transaction)).await {
        Ok(tx) => transaction = tx.unwrap(),
        Err(res) => return res,
    }

    // add users
    let fids = body
        .freshman_users
        .iter()
        .map(|a| a.fid)
        .collect::<Vec<_>>();
    let batch_ids = vec![id; fids.len()];
    match log_query(query!("INSERT INTO freshman_batch_users(fid, batch_id) SELECT fid, batch_id FROM UNNEST($1::int4[], $2::int4[]) as a(fid, batch_id)", fids.as_slice(), batch_ids.as_slice()).execute(&state.db).await.map(|_| ()), Some(transaction)).await {
        Ok(tx) => transaction = tx.unwrap(),
        Err(res) => return res,
    }

    let uids = body
        .member_users
        .iter()
        .map(|a| a.uid.clone())
        .collect::<Vec<_>>();
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

#[post("/evals/batch/pull/{user}")]
pub async fn pull_user(path: Path<(String,)>, state: Data<AppState>) -> impl Responder {
    let (user,) = path.into_inner();
    log!(Level::Info, "POST /evals/batch/pull/{user}");
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
        match log_query(
            query!("DELETE FROM freshman_batch_pulls WHERE fid = $1", user)
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
                "INSERT INTO freshman_batch_pulls(fid, approved) VALUES ($1, true)",
                user
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
    } else {
        match log_query(
            query!("DELETE FROM member_batch_pulls WHERE uid = $1", user)
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
                "INSERT INTO member_batch_pulls(uid, approved) VALUES ($1, true)",
                user
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
    }

    // Commit transaction
    match transaction.commit().await {
        Ok(_) => HttpResponse::Created().finish(),
        Err(e) => {
            log!(Level::Error, "Transaction failed to commit");
            HttpResponse::InternalServerError().body(e.to_string())
        }
    }
}

#[post("/batch/pr/{puller}/{user}")]
pub async fn submit_batch_pr(
    path: Path<(String, String)>,
    state: Data<AppState>,
    body: Json<String>,
) -> impl Responder {
    let (puller, user) = path.into_inner();
    log!(Level::Info, "POST /evals/batch/pr/{puller}/{user}");
    let reason = body.into_inner();
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
        match log_query(query!("INSERT INTO freshman_batch_pulls(fid, approved, puller, reason) VALUES ($1, false, $2, $3) ON CONFLICT DO NOTHING", user, puller, reason).execute(&state.db).await.map(|_| ()), Some(transaction)).await {
            Ok(tx) => transaction = tx.unwrap(),
            Err(res) => return res,
        }
    } else {
        match log_query(query!("INSERT INTO member_batch_pulls(uid, approved, puller, reason) VALUES ($1, false, $2, $3) ON CONFLICT DO NOTHING", user, puller, reason).execute(&state.db).await.map(|_| ()), Some(transaction)).await {
            Ok(tx) => transaction = tx.unwrap(),
            Err(res) => return res,
        }
    }

    // Commit transaction
    match transaction.commit().await {
        Ok(_) => HttpResponse::Created().finish(),
        Err(e) => {
            log!(Level::Error, "Transaction failed to commit");
            HttpResponse::InternalServerError().body(e.to_string())
        }
    }
}

#[get("/batch/pr")]
pub async fn get_pull_requests(state: Data<AppState>) -> impl Responder {
    log!(Level::Info, "GET /evals/batch/pr");
    let mut result = PullRequests {
        frosh: Vec::new(),
        members: Vec::new(),
    };
    match log_query_as(
        query_as!(
            FreshmanPull,
            "select fid, reason, puller from freshman_batch_pulls where approved = false"
        )
        .fetch_all(&state.db)
        .await,
        None,
    )
    .await
    {
        Ok((_, i)) => result.frosh = i,
        Err(res) => return res,
    }
    match log_query_as(
        query_as!(
            MemberPull,
            "select uid, reason, puller from member_batch_pulls where approved = false"
        )
        .fetch_all(&state.db)
        .await,
        None,
    )
    .await
    {
        Ok((_, i)) => result.members = i,
        Err(res) => return res,
    }

    HttpResponse::Ok().json(result)
}

#[get("/batch")]
pub async fn get_batches(state: Data<AppState>) -> impl Responder {
    log!(Level::Info, "GET /evals/batch");
    let intros: Vec<IntroStatus>;
    match get_intro_member_evals(&state).await {
        Ok(is) => {
            intros = is;
        }
        Err(e) => return e,
    }
    // return HttpResponse::Ok().json(intros);
    let (((name, uid), fid), ((seminars, directorships), (missed_hms, packet))): (
        ((Vec<String>, Vec<Option<String>>), Vec<i32>),
        ((Vec<i64>, Vec<i64>), (Vec<i64>, Vec<i64>)),
    ) = intros
        .into_iter()
        .map(|is| {
            (
                ((is.name, is.uid), is.fid.unwrap_or(0)),
                (
                    (is.seminars, is.directorships),
                    (is.missed_hms, 100 * is.signatures / is.max_signatures),
                ),
            )
        })
        .unzip();

    use serde::{Deserialize, Serialize};
    use utoipa::ToSchema;
    #[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
    struct Chom {
        bid: Option<i32>,
        mname: Option<String>,
        uid: Option<String>,
        fid: Option<i32>,
        bu: Option<bool>,
    }
    match log_query_as(
      // I'm so sorry for anyone who needs to touch this ever
    query_as!(Batch,
              "
SELECT batch.name AS \"name!\", batch.uid AS \"creator!\", bi.conditions AS \"conditions!\", bi.members AS \"members!\"
FROM (SELECT cb.bid, cb.conditions, array_agg(DISTINCT concat(cb.mname, ',', cb.uid)) AS members
FROM (
SELECT batches.bid
, array_agg(concat(batches.\"condition\", ' ', batches.comparison, ' ', batches.value)) AS conditions
, batches.mname, batches.uid, batches.fid
FROM (SELECT baid.bid, baid.mname, baid.fid, baid.uid, bc.\"condition\", bc.comparison, bc.value,
CASE
	WHEN baid.bu THEN TRUE
	WHEN bc.\"condition\" = 'packet' AND bc.comparison = 'greater' THEN evals.packet > bc.value
	WHEN bc.\"condition\" = 'packet' AND bc.comparison = 'equal' THEN evals.packet = bc.value
	WHEN bc.\"condition\" = 'packet' AND bc.comparison = 'less' THEN evals.packet < bc.value
	WHEN bc.\"condition\" = 'seminar' AND bc.comparison = 'greater' THEN evals.ss > bc.value
	WHEN bc.\"condition\" = 'seminar' AND bc.comparison = 'equal' THEN evals.ss = bc.value
	WHEN bc.\"condition\" = 'seminar' AND bc.comparison = 'less' THEN evals.ss < bc.value
	WHEN bc.\"condition\" = 'committee' AND bc.comparison = 'greater' THEN evals.ds > bc.value
	WHEN bc.\"condition\" = 'committee' AND bc.comparison = 'equal' THEN evals.ds = bc.value
	WHEN bc.\"condition\" = 'committee' AND bc.comparison = 'less' THEN evals.ds < bc.value
	WHEN bc.\"condition\" = 'house' AND bc.comparison = 'greater' THEN evals.hm > bc.value
	WHEN bc.\"condition\" = 'house' AND bc.comparison = 'equal' THEN evals.hm = bc.value
	WHEN bc.\"condition\" = 'house' AND bc.comparison = 'less' THEN evals.hm < bc.value
	ELSE false
END AS cond_passed
FROM (SELECT baid.bid, baid.mname, baid.fid, baid.uid, bool_or(baid.bu) AS bu
FROM (SELECT *
FROM (SELECT fbu.batch_id, evals.name, fbu.fid, NULL AS uid, TRUE AS bu
	FROM freshman_batch_users fbu
	LEFT JOIN (
	SELECT evals._ AS uid, evals.name, evals.fid
	FROM (SELECT *
	FROM UNNEST($1::varchar[], $2::varchar[], $3::int8[], $4::int8[], $5::int8[], $6::int8[], $7::int4[])) AS evals(\"name\", _, ss, ds, hm, packet, fid)
	) evals
	ON fbu.fid = evals.fid) AS frosh_info
UNION (
	SELECT mbu.batch_id, evals.name, NULL AS fid, mbu.uid, TRUE AS bu
	FROM member_batch_users mbu 
	LEFT JOIN (
	SELECT evals._ AS uid, evals.name, evals.fid
	FROM (SELECT *
	FROM UNNEST($1::varchar[], $2::varchar[], $3::int8[], $4::int8[], $5::int8[], $6::int8[], $7::int4[])) AS evals(\"name\", _, ss, ds, hm, packet, fid)
	) evals
	ON mbu.uid = evals.uid)
UNION (
	SELECT batch.id, evals.name, CASE WHEN evals.fid != 0 THEN evals.fid ELSE NULL END, evals.uid, FALSE AS bu
	FROM batch,
		(SELECT * FROM UNNEST($1::varchar[], $2::varchar[], $3::int8[], $4::int8[], $5::int8[], $6::int8[], $7::int4[])) AS evals(\"name\", uid, ss, ds, hm, packet, fid)
)) AS baid(bid, mname, fid, uid, bu)
GROUP BY baid.bid, baid.mname, baid.fid, baid.uid) AS baid
LEFT JOIN batch_conditions bc ON bc.batch_id=baid.bid
LEFT JOIN (
	SELECT evals.uid, evals.fid, evals.ss, evals.ds, evals.hm, evals.packet
	FROM (SELECT *
	FROM UNNEST($1::varchar[], $2::varchar[], $3::int8[], $4::int8[], $5::int8[], $6::int8[], $7::int4[])) AS evals(\"name\", uid, ss, ds, hm, packet, fid)
	) evals ON evals.uid=baid.uid OR evals.fid=baid.fid
WHERE NOT EXISTS (SELECT 1 FROM freshman_batch_pulls fbp WHERE fbp.approved AND fbp.fid=baid.fid)
AND NOT EXISTS (SELECT 1 FROM member_batch_pulls mbp WHERE mbp.approved AND mbp.uid=baid.uid)) AS batches
--WHERE cond_passed
GROUP BY batches.bid, batches.mname, batches.uid, batches.fid
HAVING bool_and(batches.cond_passed)) AS cb
GROUP BY cb.bid, cb.conditions) AS bi --thats gay
LEFT JOIN batch ON bi.bid=batch.id
", &name, &uid as _, &seminars, &directorships, &missed_hms, &packet, &fid).fetch_all(&state.db).await,
    None,
  ).await {
      Ok((_, batches)) => HttpResponse::Ok().json(batches),
      Err(e) => e,
  }
}
