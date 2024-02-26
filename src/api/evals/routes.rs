use crate::api::lib::UserError;
use crate::app::AppState;
use crate::auth::CSHAuth;
use crate::ldap::{get_group_members_exact, get_user};
use crate::schema::api::{IntroStatus, MemberStatus, Packet};
use actix_web::{
    get,
    web::{Data, Path},
    HttpResponse, Responder,
};
use chrono::{Datelike, NaiveDate, Utc};
use sqlx::{query_as, query_file_as, Pool, Postgres};

type PacketNonsense = ((Vec<String>, Vec<String>), (Vec<i64>, Vec<i64>));

fn split_packet(packets: &[Packet]) -> (Vec<String>, Vec<String>, Vec<i64>, Vec<i64>) {
    let ((usernames, names), (signatures, max_signatures)): PacketNonsense = packets
        .iter()
        .map(|p| {
            (
                (
                    p.clone().username.unwrap().trim().to_owned(),
                    p.clone().name,
                ),
                (p.signatures, p.max_signatures),
            )
        })
        .unzip();
    (usernames, names, signatures, max_signatures)
}

async fn get_all_packets(packet_db: &Pool<Postgres>) -> Result<Vec<Packet>, UserError> {
    let packets = query_as::<_, Packet>(
        "SELECT us.username,
            us.name,
            (LEAST(count(sm.packet_id), 10) + upper_signs) AS signatures,
            us.max_upper + 10 AS max_signatures
        FROM(SELECT fm.name,
                    p.id,
                    p.freshman_username AS username,
                    count(su.signed) FILTER(WHERE su.signed) AS upper_signs,
                    count(su.packet_id) AS max_upper
             FROM freshman fm
             LEFT JOIN packet p ON
                 fm.rit_username = p.freshman_username
             LEFT JOIN signature_upper su ON
                 p.id = su.packet_id
             WHERE p.freshman_username IS NOT NULL
             GROUP BY p.id, fm.name) AS us
         LEFT JOIN signature_misc sm ON
             us.id = sm.packet_id
         GROUP BY us.username, upper_signs, us.id, us.max_upper, us.name",
    )
    .fetch_all(packet_db)
    .await?;

    Ok(packets)
}

async fn get_intro_evals_status(
    packets: &[Packet],
    block: i32,
    conditional_db: &Pool<Postgres>,
) -> Result<Vec<IntroStatus>, UserError> {
    let (usernames, names, signatures, max_signatures) = split_packet(packets);
    let attendance = query_file_as!(
        IntroStatus,
        "src/queries/get_intro_evals_status.sql",
        &usernames,
        &names,
        &signatures,
        &max_signatures,
        block,
    )
    .fetch_all(conditional_db)
    .await?;
    Ok(attendance)
}

async fn get_member_evals_status(
    uids: &[String],
    names: &[String],
    conditional_db: &Pool<Postgres>,
) -> Result<Vec<MemberStatus>, UserError> {
    let now = Utc::now();
    let attendance = query_file_as!(
        MemberStatus,
        "src/queries/get_member_evals_status.sql",
        if now.month() > 5 {
            NaiveDate::from_ymd_opt(now.year(), 6, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
        } else {
            NaiveDate::from_ymd_opt(now.year() - 1, 6, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
        },
        uids,
        names,
    )
    .fetch_all(conditional_db)
    .await?;
    Ok(attendance)
}

pub async fn get_intro_member_evals_helper(
    state: &Data<AppState>,
) -> Result<Vec<IntroStatus>, UserError> {
    get_intro_evals_status(
        &get_all_packets(&state.packet_db).await?,
        state.eval_block_id,
        &state.db,
    )
    .await
}

#[utoipa::path(
    context_path="/api/evals",
    tag = "Evals",
    responses(
        (status = 200, description = "Get all current freshmen evals status", body = [IntroStatus]),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    ),
    security(
        ("csh" = [])
    )
)]
#[get("/intro", wrap = "CSHAuth::member_only()")]
pub async fn get_intro_member_evals(state: Data<AppState>) -> Result<impl Responder, UserError> {
    let attendance = get_intro_member_evals_helper(&state).await?;
    Ok(HttpResponse::Ok().json(attendance))
}

#[utoipa::path(
    context_path="/api/evals",
    tag = "Evals",
    responses(
        (status = 200, description = "Get all current member evals status", body = [MemberStatus]),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    ),
    security(
        ("csh" = [])
    )
)]
#[get("/member", wrap = "CSHAuth::member_only()")]
pub async fn get_member_evals(state: Data<AppState>) -> Result<impl Responder, UserError> {
    let (uids, names): (Vec<String>, Vec<String>) = get_group_members_exact(&state.ldap, "active")
        .await
        .map_err(|_| UserError::ServerError)?
        .iter()
        .map(|x| (x.uid.clone(), x.cn.clone()))
        .unzip();
    Ok(HttpResponse::Ok().json(get_member_evals_status(&uids, &names, &state.db).await?))
}

#[utoipa::path(
    context_path="/api/evals",
    tag = "Evals",
    responses(
        // (status = 200, description = "Get conditional"),
        // (status = 400, description = "Bad Request"),
        // (status = 401, description = "Unauthorized"),
        // (status = 500, description = "Internal Server Error"),
        (status = 418, description = "I'm a teapot"),
    ),
    security(
        ("csh" = [])
    )
)]
#[get("/conditional", wrap = "CSHAuth::member_only()")]
pub async fn get_conditional() -> impl Responder {
    HttpResponse::ImATeapot()
}

#[utoipa::path(
    context_path="/api/evals",
    tag = "Evals",
    responses(
        (status = 200, description = "Get gatekeep status for a specific user", body = MemberStatus),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error"),
    ),
    security(
        ("csh" = [])
    )
)]
#[get("/gatekeep/{user}", wrap = "CSHAuth::member_only()")]
pub async fn get_gatekeep(
    path: Path<(String,)>,
    state: Data<AppState>,
) -> Result<impl Responder, UserError> {
    let (user,) = path.into_inner();
    let (uids, names): (Vec<String>, Vec<String>) = get_user(&state.ldap, &user)
        .await
        .map_err(|_| UserError::ServerError)?
        .iter()
        .map(|u| (u.uid.clone(), u.cn.clone()))
        .unzip();
    let status = get_member_evals_status(&uids, &names, &state.db).await?;

    Ok(HttpResponse::Ok().json(status))
}
