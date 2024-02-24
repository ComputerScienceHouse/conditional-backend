// Define API Schema that the API routes will deliver to the frontend
// These are explicitly different from the DB schema. As, for example,
// directorship attendance may be relayed to the fronted as a list of member
// names / usernames, while directorship attendance is stored in the database
// as relations in one of two tables

use chrono::{NaiveDate, NaiveDateTime};

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

use super::db::{
    AttendanceStatus, BatchComparison, BatchCondition, EvalStatusEnum, MajorProjectStatusEnum,
    MeetingType, SemesterEnum,
};

#[derive(Serialize, Deserialize)]
pub struct EvalsHmAtt {
    pub attendance_status: AttendanceStatus,
    pub excuse: Option<String>,
    pub date: NaiveDate,
}

#[derive(FromRow, Serialize, Deserialize, Clone, Debug, ToSchema, PartialEq, Eq)]
pub struct User {
    /// User ID of the member
    pub uid: i32,
    /// Name of the intro member
    pub name: String,
    /// RIT username of the member
    pub rit_username: String,
    /// CSH username of the member, if they have one
    pub csh_username: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct IntroStatus {
    /// User ID of the member
    pub uid: i32,
    /// Name of the member
    pub name: String,
    /// RIT username
    pub username: String,
    /// Number of seminars attended
    pub seminars: Option<i64>,
    /// Number of directorships attended
    pub directorships: Option<i64>,
    /// Number of house meetings missed
    pub missed_hms: Option<i64>,
    /// Number of upperclassmen packet signatures recieved
    pub signatures: Option<i64>,
    /// Number of upperclassmen packet signatures for 100%
    pub max_signatures: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema, sqlx::FromRow)]
pub struct Packet {
    /// Intro member's rit username
    pub username: Option<String>,
    /// Name of the intro member
    pub name: String,
    /// Number of upperclassmen packet signatures recieved
    pub signatures: i64,
    /// Number of upperclassmen packet signatures for 100%
    pub max_signatures: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct MemberStatus {
    /// User ID of the member
    pub uid: i32,
    /// Name of the member
    pub name: String,
    /// CSH username
    pub username: String,
    /// Number of seminars attended
    pub seminars: Option<i64>,
    /// Number of directorships attended
    pub directorships: Option<i64>,
    /// Number of house meetings missed
    pub missed_hms: Option<i64>,
    /// Number of major projects passed
    pub major_projects: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct Seminar {
    /// Name of the technical seminar
    pub name: String,
    /// Date this seminar occured
    pub timestamp: chrono::NaiveDateTime,
    /// List of member usernames who attended
    pub members: Option<Vec<String>>,
    /// List of freshmen IDs who attended
    pub frosh: Option<Vec<i32>>,
    /// Whether the seminar has been approved
    pub approved: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema, sqlx::FromRow)]
pub struct Meeting {
    /// ID of the meeting
    pub id: i32,
    /// Type of the meeting
    pub meeting_type: MeetingType,
    /// Date the meeting occured
    pub timestamp: chrono::NaiveDateTime,
    /// Name of the meeting
    pub name: String,
    /// If the meeting has been approved
    pub approved: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema, sqlx::FromRow)]
pub struct MeetingAttendance {
    /// Meeting the attendance is associated with
    pub meeting: Meeting,
    /// List of [Users](User) that attended
    pub attendees: Vec<User>,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct NewIntroMember {
    pub name: String,
    pub eval_date: chrono::NaiveDate,
    pub onfloor_status: bool,
    pub room_number: Option<String>,
    pub rit_username: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct FreshmanUpgrade {
    pub uid: i32,
    pub ipa_unique_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct HouseMeetingAttendance {
    pub uid: i32,
    pub att_status: AttendanceStatus,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct HouseMeetingAttendanceUpdate {
    pub uid: i32,
    pub att_status: AttendanceStatus,
    pub excuse: String,
    pub meeting_id: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct HouseAttendance {
    pub date: NaiveDate,
    pub attendees: Vec<HouseMeetingAttendance>,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct MajorProjectSubmission {
    /// Username of member who submitted this major project
    pub uid: String,
    /// Name of this major project
    pub name: String,
    /// Description of this major project
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct MajorProjectSubmissionEboard {
    /// Username of member who submitted this major project
    pub uid: String,
    /// Name of this major project
    pub name: String,
    /// Description of this major project
    pub description: Option<String>,
    /// idk something fs
    pub status: MajorProjectStatusEnum,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct IntroForm {
    /// Social events attended
    pub social_events: String,
    /// Other comments
    pub other_comments: String,
    /// Status for user passing 6 weeks
    pub status: EvalStatusEnum,
}

pub struct CoopSubmission {
    pub uid: String,
    pub date: NaiveDateTime,
    pub semester: SemesterEnum,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct BatchConditionSubmission {
    pub value: i32,
    pub condition: BatchCondition,
    pub comparison: BatchComparison,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct FreshmanBatchSubmission {
    pub fid: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct BatchSubmission {
    pub name: String,
    pub conditions: Vec<BatchConditionSubmission>,
    pub freshman_users: Vec<FreshmanBatchSubmission>,
    pub users: Vec<i32>,
}
#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct FreshmanPull {
    pub fid: i32,
    pub reason: String,
    pub puller: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct MemberPull {
    pub uid: String,
    pub reason: String,
    pub puller: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct PullRequests {
    pub frosh: Vec<FreshmanPull>,
    pub members: Vec<MemberPull>,
}

#[derive(FromRow, Serialize, Deserialize, Clone, Debug, ToSchema, PartialEq, Eq)]
pub struct Absences {
    uid: i32,
    count: Option<i64>,
}

#[derive(FromRow, Serialize, Deserialize, Clone, Debug, ToSchema, PartialEq, Eq)]
pub struct DateWrapper {
    date: NaiveDate,
}

#[derive(FromRow, Serialize, Deserialize, Clone, Debug, ToSchema, PartialEq, Eq)]
pub struct AbsenceWrapper {
    date: NaiveDate,
    excuse: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct Batch {
    /// ID of the batch
    pub id: i32,
    /// Name of the batch
    pub name: String,
    /// User ID of the creator
    pub creator: User,
    /// A vector of conditions formatted "{condition} {comparison} {value}"
    pub conditions: Vec<String>,
    /// A vector of two comma separated values, name and CSH username.
    /// If the user doesn't have an account, the second value will be empty.
    pub members: Vec<String>,
}
