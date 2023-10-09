use ::chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::types::chrono;
use sqlx::FromRow;
use utoipa::ToSchema;

/// Enum used for 'committee_meetings' to indicate directorship type
#[derive(sqlx::Type, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Copy, ToSchema)]
#[sqlx(type_name = "committees_enum")]
pub enum CommitteeType {
    Evaluations,
    History,
    Social,
    #[serde(rename = "OpComm")]
    Opcomm,
    #[sqlx(rename = "R&D")]
    #[serde(rename = "Research and Development")]
    Rnd,
    #[sqlx(rename = "House Improvements")]
    #[serde(rename = "House Improvements")]
    Imps,
    Financial,
    /// Unused (?), but valid within the API
    Chairman,
    #[sqlx(rename = "Ad-Hoc")]
    #[serde(rename = "Ad-Hoc")]
    Adhoc,
    #[sqlx(rename = "Public Relations")]
    #[serde(rename = "Public Relations")]
    PR,
}

// ----------- ENTERING POSTGRES BULLSHIT. BLAME jmf FOR THIS -----------------

/// Enum used for 'conditional' to indicate P/F status
#[derive(sqlx::Type, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Copy)]
#[sqlx(type_name = "conditional_enum")]
pub enum ConditionalStatus {
    Pending,
    Passed,
    Failed,
}

/// Enum used for freshman project (deprecated) in 'freshman_eval_data'
#[derive(sqlx::Type, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Copy)]
#[sqlx(type_name = "freshman_project_enum")]
pub enum FreshmanProjectStatus {
    Pending,
    Passed,
    Failed,
}

/// Enum used for freshman eval status in 'freshman_eval_data'
#[derive(sqlx::Type, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Copy)]
#[sqlx(type_name = "freshman_eval_enum")]
pub enum FreshmanEvalStatus {
    Pending,
    Passed,
    Failed,
}

/// Enum used for major project status in 'major_projecs'
#[derive(sqlx::Type, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Copy)]
#[sqlx(type_name = "major_project_enum")]
pub enum MajorProjectStatus {
    Pending,
    Passed,
    Failed,
}

#[derive(sqlx::Type, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Copy)]
#[sqlx(type_name = "spring_eval_emum")]
pub enum SpringEvalStatus {
    Pending,
    Passed,
    Failed,
}

// --------- END POSTGRES BULLSHIT. BLAME joeneil FOR THE REST OF THIS --------

/// Enum used for coop semester in 'current_coops'
#[derive(sqlx::Type, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Copy)]
#[sqlx(type_name = "co_op_enum")]
pub enum CoopSemester {
    Fall,
    Spring,
    Neither,
}

/// Enum used to attendance in 'freshman_hm_attendance' and
/// 'member_hm_attendance'
#[derive(sqlx::Type, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Copy)]
#[sqlx(type_name = "attendance_enum")]
pub enum AttendanceStatus {
    Attended,
    Absent,
    Excused,
}

/// Directorship Attendance struct, represents all directorship attendance rows
/// that exist, without information as to which members attended.
///
/// Represents a row in the 'committee_meetings' table
#[derive(FromRow, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Committee {
    /// Unique id identifying a DirectorshipAttendance
    pub id: i32,
    /// The 'committee' or Directorship associated with this attendance.
    /// TODO: Should be enum?
    pub committee: String,
    /// What date the directorship occured, stored as a naive date/time
    /// (i.e. without TZ info)
    pub timestamp: chrono::NaiveDateTime,
    /// Whether the attendance is 'active'. I'm not sure what this does in
    /// original conditional, but it's in the db schema so.
    pub active: Option<bool>,
    /// Whether the attendance has been approved by an Eboard member
    pub approved: bool,
}

/// Conditional given to a member at a spring evals to avoid failing them,
/// instead giving them criteria they must meet before a specified date to
/// remain a member.
///
/// Represents a row in the 'conditional' table
#[derive(FromRow, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Conditional {
    /// Unique id identifying this Conditional
    pub id: i32,
    /// Username of the member this conditional has been assigned to
    pub uid: String,
    /// The terms of this conditional
    pub description: String,
    /// The date the conditional was created
    pub date_created: chrono::NaiveDate,
    /// The date the conditional is due (typically 1st eboard meeting,
    /// for example)
    pub date_due: chrono::NaiveDate,
    /// Whether the conditional is currently active.
    pub active: bool,
    /// Whether the conditional has passed, failed, or is still pending.
    pub status: ConditionalStatus,
    /// foreign key into 'freshman_eval_data' table
    pub i_evaluation: Option<usize>,
    /// foreign key into 'spring_evals' table, NULL in every instance in the
    /// dev database.
    pub s_evaluation: Option<usize>,
}

/// Represents a row in the 'current_coops' table
#[derive(FromRow, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Coop {
    /// Unique id identifying this Coop
    pub id: i32,
    /// Username of the member who submitted this Coop
    pub uid: String,
    /// When this coop form was submitted
    pub date_created: chrono::NaiveDate,
    /// Which semester (or neither) this coop is for.
    pub semester: CoopSemester,
}

/// Row in the freshman_accounts table
#[derive(FromRow, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct FreshmanAccount {
    /// Unique id identifying this freshman account (may be the same as packet
    /// id? TODO: Confirm). lmao no bozo
    /// that would be too easy
    pub id: i32,
    /// The legal name of the freshman
    pub name: String,
    /// TODO: Figure out what this is?
    /// might be 6weeks date but its wrong in the db
    pub eval_date: chrono::NaiveDate,
    /// Whether the freshman lives on floor. If this is true,
    /// then room_number will be Some(_).
    pub onfloor_status: Option<bool>,
    /// Which room the freshman lives in, if the live on floor.
    pub room_number: Option<String>,
    /// Number of packet signatures missed by the freshman.
    pub signatures_missed: Option<i32>,
    /// The freshman's RIT username. Should always be filled in, but it an
    /// option as there are some rows with NULL entries.
    pub rit_username: Option<String>,
}

/// Row in the 'freshman_committee_attendance' table
#[derive(FromRow, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct FreshmanCommitteeAttendance {
    /// Unique id identifying this freshman's attendance
    pub id: i32,
    /// Foreign key into 'freshman_accounts' table for freshman ids
    pub fid: i32,
    /// Foreign key into 'committee_meetings' table for attendance ids
    pub meeting_id: i32,
}

/// Row in the 'freshman_eval_data' table
#[derive(FromRow, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct FreshmanEvaluation {
    ///  Unique id for this eval information
    pub id: i32,
    /// Username of the freshman in question
    pub uid: String,
    /// Whether the freshman passed for their contribution to the freshman
    /// major project. This column is deprecated and only exists on rows for
    /// freshman prior to the elimination of the freshman project.
    pub freshman_project: Option<FreshmanProjectStatus>,
    /// The date the freshman was / will be voted on (i.e. 10 weeks / 6 weeks
    /// data). This appears to be incorrect for 2023 freshman. Did 6 weeks get
    /// rescheduled to be a week later to not conflict with fall break?
    pub eval_date: chrono::NaiveDateTime,
    /// The number of signatures that the freshman did not get on packet
    /// TODO: add column to also include total signatures available?
    pub signatures_missed: i32,
    /// The social events attended by this freshman, as submitted in the
    /// intro evals form
    pub social_events: Option<String>,
    /// Other notes entered by the freshman, as submitted in the intro evals
    /// form
    pub other_notes: Option<String>,
    /// Whether the freshman has passed / failed or is still pending
    pub freshman_eval_result: FreshmanEvalStatus,
    /// Unknown. Usually true. TODO: Ask Jeremy if he knows what this might be
    /// because it's on a lot of tables and I don't know what it does anywhere
    /// it does nothing lmao, im just making it true always
    pub active: Option<bool>,
}

/// Row in 'freshman_hm_attendance'
#[derive(FromRow, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct FreshmanHouseAttendance {
    /// Unique id for this freshman HM attendance
    pub id: i32,
    /// Foreign key into 'freshman_accounts' table
    pub fid: i32,
    /// Foreign key into 'house_meetings' table
    pub meeting_id: i32,
    /// Optional string explaining why a freshman was not at house meeting if
    /// they had an excusable reason.
    pub excuse: Option<String>,
    /// Whether the freshman was present, absent, or excused from attendance
    pub attendance_status: Option<AttendanceStatus>,
}

/// Row in 'freshman_hm_attendance'
#[derive(FromRow, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct FreshmanSeminarAttendance {
    /// Unique id for this freshman seminar attendance
    pub id: i32,
    /// Foreign key into 'freshman_accounts' table
    pub fid: i32,
    /// Foreign key into 'technical_seminars' table
    pub seminar_id: i32,
}

/// Row in 'house_meetings'
#[derive(FromRow, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct HouseMeeting {
    /// Unique id for this house meeting
    pub id: i32,
    /// Date of this house meeting
    pub date: chrono::NaiveDate,
    /// Whether this house meeting is 'active' (?)
    pub active: bool,
}

/// Row in 'in_housing_queue'
#[derive(FromRow, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct InHousingQueue {
    /// Username of member in housing queue
    pub uid: String,
}

/// Row in 'major_projects'
#[derive(FromRow, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct MajorProject {
    /// Unique id for this major project
    pub id: i32,
    /// Username of member who submitted this major project
    pub uid: String,
    /// Name of this major project
    pub name: String,
    /// Description of this major project
    pub description: Option<String>,
    /// Whether this project is 'active' (?)
    pub active: bool,
    /// Whether this project has been passed, failed, or is pending
    pub status: MajorProjectStatus,
    /// Date this major project was submitted
    pub date: NaiveDate,
}

/// Row in 'member_committee_attendance'
#[derive(FromRow, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct MemberCommitteeAttendance {
    /// Unique id for this directorship attendance
    pub id: i32,
    /// Username of member who attended this meeting
    pub uid: String,
    /// Foreign key into 'committee_meetings'
    pub meeting_id: i32,
}

/// Row in 'member_hm_attendance'
#[derive(FromRow, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct MemberHouseAttendance {
    /// Unique id for this house meeting attendance
    pub id: i32,
    /// Username of member who attended / didn't attend this house meeting
    pub uid: String,
    /// Foreign key into 'house_meetings'
    pub meeting_id: i32,
    /// Optional excuse if a member was excused from a house meeting
    pub excuse: Option<String>,
    /// Whether the member attended, was abssent, or was excused from a house
    /// meeting
    pub attendance_status: Option<AttendanceStatus>,
}

/// Row in 'member_seminar_attendance'
#[derive(FromRow, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct MemberSeminarAttendance {
    /// Unique id for this seminar attendance
    pub id: i32,
    /// Username of member who attended this seminar
    pub uid: String,
    /// Foreign key into 'technical_seminars'
    pub seminar_id: i32,
}

/// Row in 'onfloor_datetime'
#[derive(FromRow, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct OnFloorDate {
    /// Username of member granted on floor status
    pub uid: String,
    /// Time of when user was granted on floor status
    pub onfloor_granted: chrono::NaiveDateTime,
}

/// Row in 'spring_evals' table
#[derive(FromRow, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct MemberEvaluation {
    /// Unique id for this Member Evaluation
    pub id: i32,
    /// Username of member being evaluated
    pub uid: String,
    /// (?)
    pub active: bool,
    /// Date this member evaluation was created
    pub date_created: chrono::NaiveDate,
    /// Whether this member has passed, failed, or is pending for this spring
    /// evals
    pub status: SpringEvalStatus,
}

/// Row in 'technical_seminars' table
pub struct Seminar {
    /// Unique id for this technical seminar
    pub id: i32,
    /// Name of the technical seminar
    pub name: String,
    /// Date this seminar occured
    pub timestamp: chrono::NaiveDateTime,
    /// (?)
    pub active: Option<bool>,
    /// Whether this seminar attendance has been approved by an eboard member
    pub approved: bool,
}
