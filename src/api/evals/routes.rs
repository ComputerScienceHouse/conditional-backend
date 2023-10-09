use crate::api::log_query_as;
use crate::app::AppState;
use crate::ldap::{get_active_upperclassmen, get_intro_members, get_user};
use crate::schema::api::{IntroStatus, MemberStatus, Packet};
use actix_web::{
    get,
    web::{Data, Path},
    HttpResponse, Responder,
};
use log::{log, Level};
use sqlx::{query_as, Pool, Postgres};

fn split_packet(packets: &Vec<Packet>) -> (Vec<String>, Vec<String>, Vec<i64>, Vec<i64>) {
    let ((usernames, names), (signatures, max_signatures)): (
        (Vec<String>, Vec<String>),
        (Vec<i64>, Vec<i64>),
    ) = packets
        .iter()
        .map(|p| {
            (
                // (p.clone().username.unwrap(), "".to_owned()),
                // (0, 0),
                (
                    p.clone().username.unwrap().trim().to_owned(),
                    p.clone().name.unwrap(),
                ),
                (p.signatures.unwrap(), p.max_signatures.unwrap()),
            )
        })
        .unzip();
    (usernames, names, signatures, max_signatures)
}

async fn get_all_packets(packet_db: &Pool<Postgres>) -> Result<Vec<Packet>, HttpResponse> {
    match log_query_as(
        query_as::<_, Packet>(
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
        .await,
        None,
    )
    .await
    {
        Ok((_, ps)) => Ok(ps),
        Err(e) => Err(e),
    }
}

async fn get_freshmen_sdm(
    packets: &Vec<Packet>,
    conditional_db: &Pool<Postgres>,
) -> Result<Vec<IntroStatus>, HttpResponse> {
    let (usernames, names, signatures, max_signatures) = split_packet(packets);
    match log_query_as(
        query_as!(
            IntroStatus,
            "SELECT packet.name as \"name!\",
NULL as uid,
                    status.seminars as \"seminars!\",
                    status.directorships as \"directorships!\",
                    status.missed_hms as \"missed_hms!\",
                    packet.signatures as \"signatures!\",
                    packet.max_signatures as \"max_signatures!\"
                FROM (SELECT sd.username,
                        sd.seminars,
                        sd.directorships,
                        count(fha.attendance_status)
                            FILTER(WHERE fha.attendance_status = 'Absent') AS missed_hms
                    FROM (SELECT s.username,
                                 s.fid,
                                 s.seminars,
                                 count(cm.approved)
                                    FILTER(WHERE cm.approved) AS directorships
                          FROM (SELECT fa.rit_username as username,
                                       fa.id AS fid,
                                       count(ts.approved)
                                           FILTER(WHERE ts.approved) AS seminars
                                FROM freshman_accounts fa
                                LEFT JOIN freshman_seminar_attendance fsa ON
                                    fa.id = fsa.fid
                                LEFT JOIN technical_seminars ts ON
                                    fsa.seminar_id = ts.id
                                GROUP BY fa.rit_username, fa.id) AS s
                          LEFT JOIN freshman_committee_attendance fca ON
                              s.fid = fca.fid
                          LEFT JOIN committee_meetings cm ON
                              fca.meeting_id = cm.id
                          GROUP BY s.username, s.fid, s.seminars) AS sd
                    LEFT JOIN freshman_hm_attendance fha ON
                        sd.fid = fha.fid
                    GROUP BY sd.username,
                             sd.fid,
                             sd.seminars,
                             sd.directorships) AS status
                    LEFT JOIN UNNEST($1::varchar[], $2::varchar[], $3::int8[], $4::int8[]) AS packet(username, name, signatures, max_signatures) ON
                        packet.username = status.username
                    WHERE packet.name IS NOT NULL
                        AND status.seminars IS NOT NULL
                        AND status.directorships IS NOT NULL
                        AND status.missed_hms IS NOT NULL
                        AND packet.signatures IS NOT NULL
                        AND packet.max_signatures IS NOT NULL
",
        &usernames, &names, &signatures, &max_signatures)
        .fetch_all(conditional_db)
        .await,
        None,
    )
    .await
    {
        Ok((_, intros)) => {
          Ok(intros)
        }
        Err(e) => Err(e),
    }
}

async fn get_intro_member_sdm(
    uids: &Vec<String>,
    rit_usernames: &Vec<String>,
    packets: &Vec<Packet>,
    conditional_db: &Pool<Postgres>,
) -> Result<Vec<IntroStatus>, HttpResponse> {
    let (usernames, names, signatures, max_signatures) = split_packet(packets);
    match log_query_as(
        query_as!(
            IntroStatus,
          "
SELECT packet.name as \"name!\", status.uid as \"uid!\",
                    status.seminars as \"seminars!\",
                    status.directorships as \"directorships!\",
                    status.missed_hms as \"missed_hms!\",
                    packet.signatures as \"signatures!\",
                    packet.max_signatures as \"max_signatures!\"
FROM (SELECT sd.uid, sd.rit_username, sd.seminars, sd.directorships, count(mha.attendance_status) FILTER(WHERE mha.attendance_status = 'Absent') AS missed_hms
FROM (SELECT s.uid, s.rit_username, s.seminars, count(cm.approved) FILTER(WHERE cm.approved) AS directorships
FROM (SELECT ur.uid, ur.rit_username, count(ts.approved) FILTER(WHERE ts.approved) AS seminars
FROM UNNEST($1::varchar[], $2::varchar[]) AS ur(uid, rit_username)
LEFT JOIN member_seminar_attendance msa ON msa.uid = ur.uid
LEFT JOIN technical_seminars ts ON ts.id = msa.seminar_id
GROUP BY ur.uid, ur.rit_username) AS s
LEFT JOIN member_committee_attendance mca ON mca.uid = s.uid
LEFT JOIN committee_meetings cm ON cm.id = mca.meeting_id 
GROUP BY s.uid, s.rit_username, s.seminars) AS sd
LEFT JOIN member_hm_attendance mha ON mha.uid = sd.uid
GROUP BY sd.uid, sd.rit_username, sd.seminars, sd.directorships) as status
LEFT JOIN UNNEST($3::varchar[], $4::varchar[], $5::int8[], $6::int8[]) AS packet(username, \"name\", signatures, max_signatures) ON packet.username=status.rit_username
WHERE status.uid IS NOT NULL
AND packet.name IS NOT NULL
AND status.seminars IS NOT NULL
AND status.directorships IS NOT NULL
AND status.missed_hms IS NOT NULL
AND packet.signatures IS NOT NULL
AND packet.max_signatures IS NOT NULL",
        uids, rit_usernames, &usernames, &names, &signatures, &max_signatures)
        .fetch_all(conditional_db)
        .await,
        None,
    )
    .await
    {
        Ok((_, intros)) => {
          Ok(intros)
        }
        Err(e) => Err(e),
    }
}

async fn get_member_sdm(
    uids: &Vec<String>,
    names: &Vec<String>,
    year_start: &chrono::NaiveDateTime,
    conditional_db: &Pool<Postgres>,
) -> Result<Vec<MemberStatus>, HttpResponse> {
    match log_query_as(
        query_as!(
            MemberStatus,
          "
SELECT sdm.uid AS \"uid!\",
                    sdm.name as \"name!\",
                    sdm.seminars as \"seminars!\",
                    sdm.directorships as \"directorships!\",
                    sdm.missed_hms as \"missed_hms!\",
                    count(mp.status) FILTER(WHERE mp.status='Passed') AS \"major_projects!\"
FROM (SELECT sd.uid, sd.name, sd.seminars, sd.directorships, count(mha.attendance_status) FILTER(WHERE mha.attendance_status = 'Absent') AS missed_hms
FROM (SELECT s.uid, s.name, s.seminars, count(cm.approved) FILTER(WHERE cm.approved) AS directorships
FROM (SELECT ur.uid, ur.name, count(ts.approved) FILTER(WHERE ts.approved) AS seminars
FROM UNNEST($1::varchar[], $2::varchar[]) AS ur(uid, name)
LEFT JOIN member_seminar_attendance msa ON msa.uid = ur.uid
LEFT JOIN (SELECT * FROM technical_seminars ts WHERE ts.timestamp > $3::timestamp) ts ON ts.id = msa.seminar_id
GROUP BY ur.uid, ur.name) AS s
LEFT JOIN member_committee_attendance mca ON mca.uid = s.uid
LEFT JOIN (SELECT * FROM committee_meetings cm WHERE cm.timestamp > $3::timestamp) cm ON cm.id = mca.meeting_id 
GROUP BY s.uid, s.name, s.seminars) AS sd
LEFT JOIN member_hm_attendance mha ON mha.uid = sd.uid
LEFT JOIN (SELECT * FROM house_meetings hm WHERE hm.date > $3::timestamp) hm ON hm.id = mha.meeting_id
GROUP BY sd.uid, sd.name, sd.seminars, sd.directorships) as sdm
LEFT JOIN (SELECT * FROM major_projects mp WHERE mp.date > $3::timestamp) mp ON mp.uid = sdm.uid
GROUP BY sdm.uid, sdm.name, sdm.seminars, sdm.directorships, sdm.missed_hms",
        uids, names, year_start)
        .fetch_all(conditional_db)
        .await,
        None,
    )
    .await
    {
        Ok((_, intros)) => {
          Ok(intros)
        }
        Err(e) => Err(e),
    }
}

#[utoipa::path(
    context_path="/evals",
    responses(
        (status = 200, description = "Get all current freshmen evals status", body = [IntroStatus]),
        (status = 500, description = "Error created by Query"),
        )
    )]
#[get("/intro")]
pub async fn get_intro_evals(state: Data<AppState>) -> impl Responder {
    log!(Level::Info, "Get /evals/intro");
    let packets: Vec<Packet>;
    let mut freshmen_status: Vec<IntroStatus>;
    match get_all_packets(&state.packet_db).await {
        Ok(ps) => {
            packets = ps;
        }
        Err(e) => return e,
    };
    let (intro_uids, intro_rit_usernames): (Vec<String>, Vec<String>) =
        get_intro_members(&state.ldap)
            .await
            .iter()
            .map(|x| (x.uid.clone(), x.rit_username.clone()))
            .unzip();
    match get_freshmen_sdm(&packets, &state.db).await {
        Ok(intros) => {
            freshmen_status = intros;
        }
        Err(e) => return e,
    };
    match get_intro_member_sdm(&intro_uids, &intro_rit_usernames, &packets, &state.db).await {
        Ok(mut intros) => {
            freshmen_status.append(&mut intros);
        }
        Err(e) => return e,
    };
    HttpResponse::Ok().json(freshmen_status)
}

#[utoipa::path(
    context_path="/evals",
    responses(
        (status = 200, description = "Get all current member evals status", body = [MemberStatus]),
        (status = 500, description = "Error created by Query"),
        )
    )]
#[get("/member")]
pub async fn get_member_evals(state: Data<AppState>) -> impl Responder {
    log!(Level::Info, "Get /evals/member");
    let (uids, names): (Vec<String>, Vec<String>) = get_active_upperclassmen(&state.ldap)
        .await
        .iter()
        .map(|x| (x.uid.clone(), x.cn.clone()))
        .unzip();
    match get_member_sdm(&uids, &names, &state.year_start, &state.db).await {
        Ok(ms) => HttpResponse::Ok().json(ms),
        Err(e) => return e,
    }
}

#[utoipa::path(
    context_path="/evals",
    responses(
        (status = 200, description = "Get all evals statuses"),
        (status = 500, description = "Error created by Query"),
        )
    )]
#[get("/conditional")]
pub async fn get_conditional() -> impl Responder {
    log!(Level::Info, "Get /evals/conditional");
    HttpResponse::ImATeapot()
}

#[utoipa::path(
    context_path="/evals",
    responses(
        (status = 200, description = "Get gatekeep status for a specific user"),
        (status = 500, description = "Error created by Query"),
        )
    )]
#[get("/gatekeep/{user}")]
pub async fn get_gatekeep(path: Path<(String,)>, state: Data<AppState>) -> impl Responder {
    let (user,) = path.into_inner();
    log!(Level::Info, "GET /gatekeep/{}", user);
    let (uids, names): (Vec<String>, Vec<String>) = get_user(&state.ldap, &user)
        .await
        .iter()
        .map(|u| (u.uid.clone(), u.cn.clone()))
        .unzip();
    match get_member_sdm(&uids, &names, &state.year_start, &state.db).await {
        Ok(ms) => {
            if let Some(user) = ms.first() {
                HttpResponse::Ok().json(user)
            } else {
                HttpResponse::NotFound().body("User not found")
            }
        }
        Err(e) => e,
    }
}
