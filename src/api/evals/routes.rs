#![allow(unused_imports)]
use crate::api::log_query_as;
use crate::app::AppState;
use crate::schema::api::{IntroStatus, MemberStatus, Packet};
use actix_web::{get, web::Data, HttpResponse, Responder};
use log::{log, Level};
use sqlx::{query, query_as, Pool, Postgres, Transaction};
use utoipa::openapi::security::Http;

#[utoipa::path(
    context_path="/evals",
    responses(
        (status = 200, description = "Get all current freshmen evals status", body = [IntroStatus]),
        (status = 500, description = "Error created by Query"),
        )
    )]
#[get("/intro")]
pub async fn get_intro_evals(_state: Data<AppState>) -> impl Responder {
    log!(Level::Info, "Get /evals/intro");
    HttpResponse::NotImplemented()
}

#[utoipa::path(
    context_path="/evals",
    responses(
        (status = 200, description = "Get all current member evals status", body = [MemberStatus]),
        (status = 500, description = "Error created by Query"),
        )
    )]
#[get("/member")]
pub async fn get_member_evals(_state: Data<AppState>) -> impl Responder {
    log!(Level::Info, "Get /evals/member");
    HttpResponse::NotImplemented()
}

#[utoipa::path(
    context_path="/evals",
    responses(
        (status = 200, description = "Get all evals statuses"),
        (status = 500, description = "Error created by Query"),
        )
    )]
#[get("/conditional")]
pub async fn get_conditional(state: Data<AppState>) -> impl Responder {
    log!(Level::Info, "Get /evals/conditional");
    let packets: Vec<Packet>;
    let intros: Vec<IntroStatus>;

    match query_as::<_, Packet>(
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
    .fetch_all(&state.packet_db)
    .await
    {
        Ok(ps) => {
            packets = ps;
        }
        Err(e) => return HttpResponse::InternalServerError().body(e.to_string()),
    };
    // return HttpResponse::Ok().json(packets);

    let ((usernames, names), (signatures, max_signatures)): (
        (Vec<String>, Vec<String>),
        (Vec<i64>, Vec<i64>),
    ) = packets
        .into_iter()
        .map(|p| {
            (
                (p.username.unwrap().trim().to_owned(), p.name.unwrap()),
                (p.signatures.unwrap(), p.max_signatures.unwrap()),
            )
        })
        .unzip();

    match log_query_as(
        query_as!(
            IntroStatus,
            "SELECT packet.name as \"name!\",
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
        .fetch_all(&state.db)
        .await,
        None,
    )
    .await
    {
        Ok((_, is)) => {
            intros = is;
        }
        Err(e) => return e,
    };

    todo!();

    HttpResponse::Ok().json(intros)
    // HttpResponse::Ok().json(packets)
    // HttpResponse::NotImplemented().into()
}
