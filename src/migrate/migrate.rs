use crate::ldap::client::LdapClient;
use chrono::Datelike;
use sqlx::migrate::Migrator;
use std::collections::HashMap;
use std::env;

use crate::migrate::schema::*;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::FromRow;
use sqlx::{Pool, Postgres, Transaction};

#[derive(FromRow, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct IntroId(Option<String>);

#[derive(FromRow, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct CshUsername(String);

async fn insert_freshmen_accounts<'a>(
    old_pool: &Pool<Postgres>,
    frosh_pool: &Pool<Postgres>,
    mut transaction: Transaction<'a, Postgres>,
) -> Result<(HashMap<i32, i32>, Transaction<'a, Postgres>), sqlx::Error> {
    let mut fid_map = HashMap::new();
    let mut frosh_uuid_map = HashMap::new();

    let frosh_accounts =
        sqlx::query_as::<_, old::FreshmanAccount>("SELECT * FROM freshman_accounts")
            .fetch_all(old_pool)
            .await?;

    for acc in frosh_accounts.clone().into_iter() {
        let acct = match sqlx::query_as::<_, IntroId>(
            "SELECT id as name FROM user_entity WHERE realm_id='intro' and username=$1",
        )
        .bind(acc.rit_username.clone())
        .fetch_one(frosh_pool)
        .await
        {
            Ok(id) => id,
            Err(_) => IntroId(None),
        };
        frosh_uuid_map.insert(acc.rit_username, acct);
    }

    let frosh_accounts = frosh_accounts.into_iter().map(|old| {
        if let Some(acct) = frosh_uuid_map.get(&old.rit_username) {
            new::User {
                id: old.id,
                name: old.name,
                intro_id: acct.0.clone(),
                ipa_unique_id: None,
                rit_username: old.rit_username,
                csh_username: None,
                is_csh: false,
                is_intro: true,
            }
        } else {
            panic!()
        }
    });

    for frosh_account in frosh_accounts {
        let uid: (i32,) = sqlx::query_as(
            "INSERT INTO \"user\"(name, intro_id, ipa_unique_id, rit_username, csh_username, \
             is_csh, is_intro) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
        )
        .bind(frosh_account.name)
        .bind(frosh_account.intro_id)
        .bind(frosh_account.ipa_unique_id)
        .bind(frosh_account.rit_username)
        .bind(frosh_account.csh_username)
        .bind(frosh_account.is_csh)
        .bind(frosh_account.is_intro)
        .fetch_one(&mut *transaction)
        .await?;
        fid_map.insert(frosh_account.id, uid.0);
    }
    Ok((fid_map, transaction))
}

async fn insert_upperclassmen_accounts<'a>(
    old_pool: &Pool<Postgres>,
    frosh_pool: &Pool<Postgres>,
    mut transaction: Transaction<'a, Postgres>,
) -> Result<(HashMap<String, i32>, Transaction<'a, Postgres>), sqlx::Error> {
    let mut username_map = HashMap::new();
    let ldap = LdapClient::new(
        env::var("CONDITIONAL_LDAP_BIND_DN")
            .expect("CONDITIONAL_LDAP_BIND_DN not set")
            .as_str(),
        env::var("CONDITIONAL_LDAP_BIND_PW")
            .expect("CONDITIONAL_LDAP_BIND_PW not set")
            .as_str(),
    )
    .await;

    let conditional_members =
        sqlx::query_as::<_, CshUsername>("SELECT DISTINCT uid FROM member_hm_attendance mha")
            .fetch_all(old_pool)
            .await?;

    let mut frosh_uuid_map = HashMap::new();

    let mut six_weeks_csh_members = match ldap.get_group_members("10weeks").await {
        Ok(members) => members,
        Err(e) => panic!("{}", e),
    };
    six_weeks_csh_members.retain(|member| {
        !member.rit_username.is_empty()
            && conditional_members.contains(&CshUsername(member.uid.clone()))
    });
    let mut upperclassmen = match ldap.get_upperclassmen().await {
        Ok(members) => members,
        Err(e) => panic!("{}", e),
    };
    upperclassmen.retain(|member| {
        !member.rit_username.is_empty()
            && conditional_members.contains(&CshUsername(member.uid.clone()))
    });

    for ldap_user in six_weeks_csh_members
        .clone()
        .into_iter()
        .chain(upperclassmen.clone().into_iter())
    {
        let acct = match sqlx::query_as::<_, IntroId>(
            "SELECT id as name FROM user_entity WHERE realm_id='intro' and username=$1",
        )
        .bind(ldap_user.rit_username.clone())
        .fetch_one(frosh_pool)
        .await
        {
            Ok(id) => id,
            Err(_) => IntroId(None),
        };
        frosh_uuid_map.insert(ldap_user.rit_username, acct.0);
    }

    // println!("{:?}", upperclassmen);
    for user in six_weeks_csh_members {
        if let Some(intro_id) = frosh_uuid_map.get(&user.rit_username) {
            let uid: (i32,) = sqlx::query_as(
                "INSERT INTO \"user\"(name, intro_id, ipa_unique_id, rit_username, csh_username, \
                 is_csh, is_intro) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
            )
            .bind(&user.cn)
            .bind(intro_id)
            .bind(&user.ipa_unique_id)
            .bind(&user.rit_username)
            .bind(&user.uid)
            .bind(true)
            .bind(true)
            .fetch_one(&mut *transaction)
            .await?;
            username_map.insert(user.uid, uid.0);
        }
    }
    for user in upperclassmen {
        if let Some(intro_id) = frosh_uuid_map.get(&user.rit_username) {
            let uid: (i32,) = sqlx::query_as(
                "INSERT INTO \"user\"(name, intro_id, ipa_unique_id, rit_username, csh_username, \
                 is_csh, is_intro) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
            )
            .bind(&user.cn)
            .bind(intro_id)
            .bind(&user.ipa_unique_id)
            .bind(&user.rit_username)
            .bind(&user.uid)
            .bind(true)
            .bind(false)
            .fetch_one(&mut *transaction)
            .await?;
            username_map.insert(user.uid, uid.0);
        }
    }
    Ok((username_map, transaction))
}

async fn insert_other_meetings<'a>(
    old_pool: &Pool<Postgres>,
    mut transaction: Transaction<'a, Postgres>,
) -> Result<
    (
        HashMap<(new::MeetingType, i32), i32>,
        Transaction<'a, Postgres>,
    ),
    sqlx::Error,
> {
    let old_directorhsips = sqlx::query_as::<_, old::Committee>("SELECT * FROM committee_meetings")
        .fetch_all(old_pool)
        .await?;
    let old_seminars = sqlx::query_as::<_, old::Seminar>("SELECT * FROM technical_seminars")
        .fetch_all(old_pool)
        .await?;
    let old_directorships = old_directorhsips.into_iter().map(|old| new::OtherMeeting {
        id: old.id,
        datetime: old.timestamp,
        name: old.committee.get_value(),
        meeting_type: new::MeetingType::Directorship,
        approved: old.approved,
    });
    let old_seminars = old_seminars.into_iter().map(|old| new::OtherMeeting {
        id: old.id,
        datetime: old.timestamp,
        name: old.name,
        meeting_type: new::MeetingType::Seminar,
        approved: old.approved,
    });
    let mut meeting_map = HashMap::new();
    for meeting in old_directorships.chain(old_seminars) {
        let new_id: (i32,) = sqlx::query_as(
            "INSERT INTO other_meeting(datetime, name, meeting_type, approved) VALUES ($1, $2, \
             $3, $4) RETURNING id",
        )
        .bind(meeting.datetime)
        .bind(meeting.name)
        .bind(meeting.meeting_type)
        .bind(meeting.approved)
        .fetch_one(&mut *transaction)
        .await?;
        meeting_map.insert((meeting.meeting_type, meeting.id), new_id.0);
    }
    Ok((meeting_map, transaction))
}

async fn insert_house_meetings<'a>(
    old_pool: &Pool<Postgres>,
    mut transaction: Transaction<'a, Postgres>,
) -> Result<(HashMap<i32, i32>, Transaction<'a, Postgres>), sqlx::Error> {
    let house_meetings = sqlx::query_as::<_, old::HouseMeeting>("SELECT * FROM house_meetings")
        .fetch_all(old_pool)
        .await?
        .into_iter()
        .map(|old| new::HouseMeeting {
            id: old.id,
            date: old.date,
        });
    let mut hm_map = HashMap::new();
    for house_meeting in house_meetings {
        let new_id: (i32,) =
            sqlx::query_as("INSERT INTO house_meeting(date) VALUES ($1) RETURNING id")
                .bind(house_meeting.date)
                .fetch_one(&mut *transaction)
                .await?;
        hm_map.insert(house_meeting.id, new_id.0);
    }
    Ok((hm_map, transaction))
}

async fn insert_coops<'a>(
    username_map: &HashMap<String, i32>,
    old_pool: &Pool<Postgres>,
    mut transaction: Transaction<'a, Postgres>,
) -> Result<Transaction<'a, Postgres>, sqlx::Error> {
    let ((_ids, uids), (dates, semesters)): (
        (Vec<i32>, Vec<i32>),
        (Vec<chrono::NaiveDate>, Vec<new::SemesterEnum>),
    ) = sqlx::query_as::<_, old::Coop>("SELECT * FROM current_coops")
        .fetch_all(old_pool)
        .await?
        .into_iter()
        .filter_map(|old| {
            if let Some(uid) = username_map.get(&old.uid) {
                Some((
                    (old.id, uid),
                    (old.date_created, old.semester.get_semester_enum()),
                ))
            } else {
                println!("inserting coops: no fk for uid {}", old.uid);
                None
            }
        })
        .unzip();
    sqlx::query(
        "INSERT INTO coop(uid, date, semester) SELECT uid, date_created, semester FROM \
         UNNEST($1::int4[], $2::date[], $3::semester_enum[]) as a(uid, date_created, semester)",
    )
    .bind(uids.as_slice())
    .bind(dates.as_slice())
    .bind(semesters.as_slice())
    .execute(&mut *transaction)
    .await?;
    // hm_map.insert(house_meeting.id, new_id.0);
    Ok(transaction)
}

async fn insert_hm_attendance<'a>(
    fid_map: &HashMap<i32, i32>,
    username_map: &HashMap<String, i32>,
    house_meeting_map: &HashMap<i32, i32>,
    old_pool: &Pool<Postgres>,
    mut transaction: Transaction<'a, Postgres>,
) -> Result<Transaction<'a, Postgres>, sqlx::Error> {
    let ((mut uids, mut meeting_ids), (mut excuse, mut attendance_status)): (
        (Vec<i32>, Vec<i32>),
        (Vec<Option<String>>, Vec<new::AttendanceStatus>),
    ) = sqlx::query_as::<_, old::MemberHouseMeetingAttendance>(
        "SELECT * FROM member_hm_attendance",
    )
    .fetch_all(old_pool)
    .await?
    .into_iter()
    .filter_map(|old| {
        if let Some(uid) = username_map.get(&old.uid) {
            if let Some(hmid) = house_meeting_map.get(&old.meeting_id) {
                Some((
                    (uid, hmid),
                    (old.excuse, old.attendance_status.get_new_value()),
                ))
            } else {
                println!(
                    "inserting member hm attendance: no fk for hmid {}",
                    old.meeting_id
                );
                None
            }
        } else {
            println!("inserting member hm attendance: no fk for uid {}", old.uid);
            None
        }
    })
    .unzip();
    let ((freshman_uids, freshman_meeting_id), (freshman_excuse, freshman_attendance_status)): (
        (Vec<i32>, Vec<i32>),
        (Vec<Option<String>>, Vec<new::AttendanceStatus>),
    ) = sqlx::query_as::<_, old::FreshmanHouseMeetingAttendance>(
        "SELECT * FROM freshman_hm_attendance",
    )
    .fetch_all(old_pool)
    .await?
    .into_iter()
    .filter_map(|old| {
        if let Some(uid) = fid_map.get(&old.fid) {
            if let Some(hmid) = house_meeting_map.get(&old.meeting_id) {
                Some((
                    (uid, hmid),
                    (old.excuse, old.attendance_status.get_new_value()),
                ))
            } else {
                println!(
                    "inserting freshman hm attendance: no fk for hmid {}",
                    old.meeting_id
                );
                None
            }
        } else {
            println!(
                "inserting freshman hm attendance: no fk for fid {}",
                old.fid
            );
            None
        }
    })
    .unzip();
    uids.extend(freshman_uids);
    meeting_ids.extend(freshman_meeting_id);
    attendance_status.extend(freshman_attendance_status);
    excuse.extend(freshman_excuse);
    sqlx::query(
        "INSERT INTO hm_attendance(uid, house_meeting_id, attendance_status, excuse) SELECT uid, \
         house_meeting_id, attendance_status, excuse FROM UNNEST($1::int4[], $2::int4[], \
         $3::hm_attendance_status_enum[], $4) as a(uid, house_meeting_id, attendance_status, \
         excuse)",
    )
    .bind(uids.as_slice())
    .bind(meeting_ids.as_slice())
    .bind(attendance_status.as_slice())
    .bind(excuse.as_slice())
    .execute(&mut *transaction)
    .await?;
    // hm_map.insert(house_meeting.id, new_id.0);
    Ok(transaction)
}

async fn insert_intro_eval_data<'a>(
    username_map: &HashMap<String, i32>,
    old_pool: &Pool<Postgres>,
    mut transaction: Transaction<'a, Postgres>,
) -> Result<Transaction<'a, Postgres>, sqlx::Error> {
    let (((uids, eval_block_ids), (social_events, other_comments)), eval_statuses): (
        (
            (Vec<i32>, Vec<i32>),
            (Vec<Option<String>>, Vec<Option<String>>),
        ),
        Vec<new::EvalStatusEnum>,
    ) = sqlx::query_as::<_, old::FreshmanEvalData>(
        "SELECT * FROM freshman_eval_data WHERE eval_date > '2023-06-01'	AND eval_date < \
         '2023-10-15'",
    )
    .fetch_all(old_pool)
    .await?
    .into_iter()
    .filter_map(|old| {
        if let Some(uid) = username_map.get(&old.uid) {
            Some((
                ((uid, 1), (old.social_events, old.other_notes)),
                old.freshman_eval_result.get_eval_enum(),
            ))
        } else {
            println!("inserting intro eval data: no fk for uid {}", old.uid);
            None
        }
    })
    .unzip();
    sqlx::query(
        "INSERT INTO intro_eval_data(uid, eval_block_id, social_events, other_comments, status) \
         SELECT uid, eval_block_id, social_events, other_comments, status FROM UNNEST($1::int4[], \
         $2::int4[], $3::text[], $4::text[], $5::eval_status_enum[]) as a(uid, eval_block_id, \
         social_events, other_comments, status)",
    )
    .bind(uids.as_slice())
    .bind(eval_block_ids.as_slice())
    .bind(social_events.as_slice())
    .bind(other_comments.as_slice())
    .bind(eval_statuses.as_slice())
    .execute(&mut *transaction)
    .await?;
    Ok(transaction)
}

async fn insert_major_projects<'a>(
    username_map: &HashMap<String, i32>,
    old_pool: &Pool<Postgres>,
    mut transaction: Transaction<'a, Postgres>,
) -> Result<Transaction<'a, Postgres>, sqlx::Error> {
    let (((uids, names), (descriptions, dates)), statuses): (
        (
            (Vec<i32>, Vec<String>),
            (Vec<String>, Vec<chrono::NaiveDate>),
        ),
        Vec<new::MajorProjectStatusEnum>,
    ) = sqlx::query_as::<_, old::MajorProject>("SELECT * FROM major_projects")
        .fetch_all(old_pool)
        .await?
        .into_iter()
        .filter_map(|old| {
            if let Some(uid) = username_map.get(&old.uid) {
                Some((
                    (
                        (uid, old.name),
                        (old.description.unwrap_or("".to_owned()), old.date),
                    ),
                    old.status.get_new_enum(),
                ))
            } else {
                println!("inserting major project: no fk for uid {}", old.uid);
                None
            }
        })
        .unzip();
    sqlx::query(
        "INSERT INTO major_project(
uid, name, description, date, status) SELECT
uid, name, description, date, status FROM UNNEST($1::int4[], $2::text[], $3::text[], $4::date[], \
         $5::major_project_status_enum[]) as a(
uid, name, description, date, status)",
    )
    .bind(uids.as_slice())
    .bind(names.as_slice())
    .bind(descriptions.as_slice())
    .bind(dates.as_slice())
    .bind(statuses.as_slice())
    .execute(&mut *transaction)
    .await?;
    Ok(transaction)
}

async fn insert_member_eval_data<'a>(
    username_map: &HashMap<String, i32>,
    old_pool: &Pool<Postgres>,
    mut transaction: Transaction<'a, Postgres>,
) -> Result<Transaction<'a, Postgres>, sqlx::Error> {
    let ((uids, years), statuses): ((Vec<i32>, Vec<i32>), Vec<new::EvalStatusEnum>) =
        sqlx::query_as::<_, old::SpringEval>("SELECT * FROM spring_evals")
            .fetch_all(old_pool)
            .await?
            .into_iter()
            .filter_map(|old| {
                if let Some(uid) = username_map.get(&old.uid) {
                    Some(((uid, old.date_created.year()), old.status.get_new_enum()))
                } else {
                    println!("inserting member eval data: no fk for uid {}", old.uid);
                    None
                }
            })
            .unzip();
    sqlx::query(
        "INSERT INTO member_eval_data(
uid, year, status) SELECT
uid, year, status FROM UNNEST($1::int4[], $2::int4[], $3::eval_status_enum[]) as a(
uid, year, status) ON CONFLICT DO NOTHING",
    )
    .bind(uids.as_slice())
    .bind(years.as_slice())
    .bind(statuses.as_slice())
    .execute(&mut *transaction)
    .await?;
    Ok(transaction)
}

async fn insert_om_attendances<'a>(
    fid_map: &HashMap<i32, i32>,
    username_map: &HashMap<String, i32>,
    meeting_map: &HashMap<(new::MeetingType, i32), i32>,
    old_pool: &Pool<Postgres>,
    mut transaction: Transaction<'a, Postgres>,
) -> Result<Transaction<'a, Postgres>, sqlx::Error> {
    let (mut uid, mut meeting_id): (Vec<i32>, Vec<i32>) =
        sqlx::query_as::<_, old::FCA>("SELECT * FROM freshman_committee_attendance")
            .fetch_all(old_pool)
            .await?
            .into_iter()
            .filter_map(|old| {
                if let Some(uid) = fid_map.get(&old.fid) {
                    if let Some(meeting_id) =
                        meeting_map.get(&(new::MeetingType::Directorship, old.meeting_id))
                    {
                        Some((uid, meeting_id))
                    } else {
                        println!(
                            "inserting fca: no fk for directorship id {}",
                            old.meeting_id
                        );
                        None
                    }
                } else {
                    println!("inserting fca: no fk for fid {}", old.fid);
                    None
                }
            })
            .unzip();
    let (temp_uid, temp_meeting_id): (Vec<i32>, Vec<i32>) =
        sqlx::query_as::<_, old::FSA>("SELECT * FROM freshman_seminar_attendance")
            .fetch_all(old_pool)
            .await?
            .into_iter()
            .filter_map(|old| {
                if let Some(uid) = fid_map.get(&old.fid) {
                    if let Some(meeting_id) =
                        meeting_map.get(&(new::MeetingType::Seminar, old.seminar_id))
                    {
                        Some((uid, meeting_id))
                    } else {
                        println!("inserting fsa: no fk for seminar id {}", old.seminar_id);
                        None
                    }
                } else {
                    println!("inserting fsa: no fk for fid {}", old.fid);
                    None
                }
            })
            .unzip();
    uid.extend(temp_uid);
    meeting_id.extend(temp_meeting_id);
    let (temp_uid, temp_meeting_id): (Vec<i32>, Vec<i32>) =
        sqlx::query_as::<_, old::MCA>("SELECT * FROM member_committee_attendance")
            .fetch_all(old_pool)
            .await?
            .into_iter()
            .filter_map(|old| {
                if let Some(uid) = username_map.get(&old.uid) {
                    if let Some(meeting_id) =
                        meeting_map.get(&(new::MeetingType::Directorship, old.meeting_id))
                    {
                        Some((uid, meeting_id))
                    } else {
                        println!("inserting mca: no fk for meeting id {}", old.meeting_id);
                        None
                    }
                } else {
                    println!("inserting mca: no fk for fid {}", old.uid);
                    None
                }
            })
            .unzip();
    uid.extend(temp_uid);
    meeting_id.extend(temp_meeting_id);
    let (temp_uid, temp_meeting_id): (Vec<i32>, Vec<i32>) =
        sqlx::query_as::<_, old::MSA>("SELECT * FROM member_seminar_attendance")
            .fetch_all(old_pool)
            .await?
            .into_iter()
            .filter_map(|old| {
                if let Some(uid) = username_map.get(&old.uid) {
                    if let Some(seminar_id) =
                        meeting_map.get(&(new::MeetingType::Seminar, old.seminar_id))
                    {
                        Some((uid, seminar_id))
                    } else {
                        println!("inserting mca: no fk for seminar id {}", old.seminar_id);
                        None
                    }
                } else {
                    println!("inserting mca: no fk for fid {}", old.uid);
                    None
                }
            })
            .unzip();
    uid.extend(temp_uid);
    meeting_id.extend(temp_meeting_id);
    sqlx::query(
        "INSERT INTO om_attendance(
  uid, om_id) SELECT
  uid, om_id FROM UNNEST($1::int4[], $2::int4[]) as a(
  uid, om_id) ON CONFLICT DO NOTHING",
    )
    .bind(uid.as_slice())
    .bind(meeting_id.as_slice())
    .execute(&mut *transaction)
    .await?;
    Ok(transaction)
}

async fn insert_conditionals<'a>(
    username_map: &HashMap<String, i32>,
    fid_map: &HashMap<i32, i32>,
    old_pool: &Pool<Postgres>,
    mut transaction: Transaction<'a, Postgres>,
) -> Result<Transaction<'a, Postgres>, sqlx::Error> {
    let (((uids, descriptions), (start_dates, due_dates)), statuses): (
        (
            (Vec<i32>, Vec<String>),
            (Vec<chrono::NaiveDate>, Vec<chrono::NaiveDate>),
        ),
        Vec<new::ConditionalStatus>,
    ) = sqlx::query_as::<_, old::Conditional>("SELECT * FROM conditional")
        .fetch_all(old_pool)
        .await?
        .into_iter()
        .filter_map(|old| {
            if let Some(uid) = username_map.get(&old.uid).or_else(|| {
                old.uid
                    .parse::<i32>()
                    .ok()
                    .and_then(|fid| fid_map.get(&fid))
            }) {
                Some((
                    ((uid, old.description), (old.date_created, old.date_due)),
                    old.status.get_conditional_status_enum(),
                ))
            } else {
                None
            }
        })
        .unzip();
    sqlx::query(
        "INSERT INTO conditional(
uid, description, start_date, due_date, status) SELECT
uid, description, start_date, due_date, status FROM UNNEST($1::int4[], $2::text[], $3::date[], \
         $4::date[], $5::conditional_status_enum[]) as a(
uid, description, start_date, due_date, status) ON CONFLICT DO NOTHING",
    )
    .bind(uids.as_slice())
    .bind(descriptions.as_slice())
    .bind(start_dates.as_slice())
    .bind(due_dates.as_slice())
    .bind(statuses.as_slice())
    .execute(&mut *transaction)
    .await?;
    Ok(transaction)
}

// id
// uid
// description
// start_date
// due_date
// status

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

async fn run_down_migrations(new_pool: &Pool<Postgres>) {
    let _ = MIGRATOR.undo(new_pool, 20240122010834).await;
}

async fn migrate() -> Result<(), Box<dyn std::error::Error>> {
    let old_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(
            "postgresql://conditional:alDeEe5vFt25QjpBpm6WhoCG2NHSYNPF@postgres.csh.rit.edu/\
             conditional",
        )
        .await?;
    // .connect("postgresql://conditionaldev:y3vNyHE9Qp9m9QQ3xTeiu3qztZKzwc@
    // postgres.csh.rit.edu/conditionaldev").await?;
    let new_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(
            "postgresql://conditionalnew:yCrhk5gF62Bu9QZyQfAVn8*jEPMxv!CS@postgres.csh.rit.edu/\
             conditionalnew",
        )
        .await?;

    let frosh_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgresql://keycloak:stimulating39;others@postgres.csh.rit.edu/keycloak")
        .await?;

    let mut transaction = new_pool.begin().await?;

    println!("started a transaction");

    // user table
    let temp = insert_freshmen_accounts(&old_pool, &frosh_pool, transaction).await?;
    println!("inserted freshmen accs");
    let fid_map = temp.0;
    transaction = temp.1;
    let temp = insert_upperclassmen_accounts(&old_pool, &frosh_pool, transaction).await?;
    let username_map = temp.0;
    transaction = temp.1;

    // (directorships, seminars) -> other_meeting table
    let temp = insert_other_meetings(&old_pool, transaction).await?;
    let meeting_map = temp.0;
    transaction = temp.1;

    // house_meetings -> house_meeting table
    let temp = insert_house_meetings(&old_pool, transaction).await?;
    let house_meeting_map = temp.0;
    transaction = temp.1;

    // current_coops -> coop table
    transaction = insert_coops(&username_map, &old_pool, transaction).await?;

    // (freshman_hm_attendance, member_hm_attendance) -> hm_attendance
    transaction = insert_hm_attendance(
        &fid_map,
        &username_map,
        &house_meeting_map,
        &old_pool,
        transaction,
    )
    .await?;

    // (freshman_committee_attendance, freshman_seminar_attendance,
    // member_committee_attendance, member_seminar_attendance) -> om_attendance
    transaction = insert_om_attendances(
        &fid_map,
        &username_map,
        &meeting_map,
        &old_pool,
        transaction,
    )
    .await?;

    // freshman_eval_data -> intro_eval_data
    transaction = insert_intro_eval_data(&username_map, &old_pool, transaction).await?;

    // major_projects -> major_project
    transaction = insert_major_projects(&username_map, &old_pool, transaction).await?;

    // spring_evals -> member_eval_data
    transaction = insert_member_eval_data(&username_map, &old_pool, transaction).await?;

    // conditional -> conditional
    transaction = insert_conditionals(&username_map, &fid_map, &old_pool, transaction).await?;

    // transaction.rollback().await?;
    transaction.commit().await?;
    // let chom2 = chom.into_iter().filter()

    Ok(())
}
