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
use sqlx::{query_as, Pool, Postgres};

fn split_packet(packets: &Vec<Packet>) -> (Vec<String>, Vec<String>, Vec<i64>, Vec<i64>) {
    let ((usernames, names), (signatures, max_signatures)): (
        (Vec<String>, Vec<String>),
        (Vec<i64>, Vec<i64>),
    ) = packets
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
    packets: &Vec<Packet>,
    block: i32,
    conditional_db: &Pool<Postgres>,
) -> Result<Vec<IntroStatus>, UserError> {
    let (usernames, names, signatures, max_signatures) = split_packet(packets);
    let attendance = query_as!(
        IntroStatus,
        r#"select
        s.name,
        s.username,
        s.uid,
        s.seminars,
        s.directorships,
        packet.signatures,
        packet.max_signatures,
        count(ha.attendance_status)
    filter(
    where
        ha.attendance_status = 'Absent') as missed_hms
    from
        (
        select
            u.name,
            u.rit_username as username,
            u.id as uid,
            count(om.approved)
       filter(
        where
            om.meeting_type = 'Seminar') as seminars,
            count(om.approved)
        filter(
        where
            om.meeting_type = 'Directorship') as directorships
        from
            "user" u
        left join om_attendance oma on
            u.id = oma.uid
        left join other_meeting om on
            oma.om_id = om.id
        left join intro_eval_data ied on
            u.id = ied.uid
        left join intro_eval_block ieb on
            ieb.id = ied.eval_block_id
        where
            ied.eval_block_id = $5 and ied.status != 'Passed' and om.datetime between ieb.start_date and ieb.end_date
        group by
            u.rit_username,
            u.id) as s
    left join hm_attendance ha on
        s.uid = ha.uid
    left join unnest($1::varchar[],
        $2::varchar[],
        $3::int8[],
        $4::int8[]) as
         packet(username,
        name,
        signatures,
        max_signatures) on
        packet.username = s.username
    where
        packet.name is not null
        and s.seminars is not null
        and s.directorships is not null
        and packet.signatures is not null
        and packet.max_signatures is not null
    group by
        s.name,
        s.username,
        s.uid,
        s.seminars,
        s.directorships,
        packet.signatures,
        packet.max_signatures"#,
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
    let attendance = query_as!(
        MemberStatus,
        r#"select
    s.name,
    s.username,
    s.uid,
    s.seminars,
    s.directorships,
    s.missed_hms,
    s.major_projects
from
    (
    select
        u.name,
        u.rit_username as username,
        u.id as uid,
        count(om.approved)
       filter(
    where
        om.meeting_type = 'Seminar') as seminars,
        count(om.approved)
        filter(
    where
        om.meeting_type = 'Directorship') as directorships,
        count(ha.attendance_status)
        filter(
        where ha.attendance_status = 'Absent') as missed_hms,
        count(mp.status)
        filter(
        where mp.status = 'Passed') as major_projects
    from
        "user" u
    left join om_attendance oma on
        u.id = oma.uid
    left join other_meeting om on
        oma.om_id = om.id
    left join hm_attendance ha on
        u.id = ha.uid
    left join major_project mp on
        u.id = mp.uid
    where
        not is_intro and om.datetime > $1
        and u.csh_username in (select UNNEST($2::varchar[]))
        and u.name in (select UNNEST($3::varchar[]))
    group by
        u.rit_username,
        u.id) as s
group by
    s.name,
    s.username,
    s.uid,
    s.seminars,
    s.directorships,
    s.missed_hms,
    s.major_projects"#,
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
    let attendance = get_intro_evals_status(
        &get_all_packets(&state.packet_db).await?,
        state.eval_block_id,
        &state.db,
    )
    .await?;
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
