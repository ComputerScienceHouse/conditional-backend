// Define API Schema that the API routes will deliver to the frontend
// These are explicitly different from the DB schema. As, for example,
// directorship attendance may be relayed to the fronted as a list of member
// names / usernames, while directorship attendance is stored in the database
// as relations in one of two tables

use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use utoipa::ToSchema;

use super::db::{
    AttendanceStatus, BatchComparison, BatchCondition, BatchConditionType, CommitteeType,
    CoopSemester, FreshmanBatchPull, FreshmanBatchUser, MajorProjectStatus, MemberBatchPull,
    MemberBatchUser,
};

pub struct ID {
    pub id: i32,
}

#[derive(Serialize, Deserialize)]
pub struct Date {
    pub date: NaiveDate,
}

#[derive(Serialize, Deserialize)]
pub struct EvalsHmAtt {
    pub attendance_status: AttendanceStatus,
    pub excuse: Option<String>,
    pub date: NaiveDate,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct IntroStatus {
    /// Name of the intro member
    pub name: Option<String>,
    /// Name of the intro member
    pub uid: Option<String>,
    /// Number of seminars attended
    pub seminars: i64,
    /// Number of directorships attended
    pub directorships: i64,
    /// Number of house meetings missed
    pub missed_hms: i64,
    /// Number of upperclassmen packet signatures recieved
    pub signatures: i64,
    /// Number of upperclassmen packet signatures for 100%
    pub max_signatures: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema, sqlx::FromRow)]
pub struct Packet {
    /// Intro member's rit username
    pub username: Option<String>,
    /// Name of the intro member
    pub name: Option<String>,
    /// Number of upperclassmen packet signatures recieved
    pub signatures: Option<i64>,
    /// Number of upperclassmen packet signatures for 100%
    pub max_signatures: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct MemberStatus {
    /// Name of the member
    pub name: String,
    /// CSH username
    pub uid: String,
    /// Number of seminars attended
    pub seminars: i64,
    /// Number of directorships attended
    pub directorships: i64,
    /// Number of house meetings missed
    pub missed_hms: i64,
    /// Number of major projects passed
    pub major_projects: i64,
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

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct Directorship {
    pub committee: CommitteeType,
    pub timestamp: chrono::NaiveDateTime,
    pub members: Option<Vec<String>>,
    pub frosh: Option<Vec<i32>>,
    pub approved: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct MeetingAttendance {
    pub name: String,
    pub date: NaiveDateTime,
    pub members: Vec<String>,
    pub frosh: Vec<i32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct DirectorshipAttendance {
    pub committee: CommitteeType,
    pub timestamp: chrono::NaiveDateTime,
    pub approved: bool,
    pub members: Vec<String>,
    pub frosh: Vec<i32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct MemberHouseAttendance {
    pub name: String,
    pub att_status: AttendanceStatus,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct FroshHouseAttendance {
    pub fid: i32,
    pub att_status: AttendanceStatus,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct HouseAttendance {
    pub date: NaiveDate,
    pub members: Vec<MemberHouseAttendance>,
    pub frosh: Vec<FroshHouseAttendance>,
}

pub struct MajorProjectSubmission {
    /// Username of member who submitted this major project
    pub uid: String,
    /// Name of this major project
    pub name: String,
    /// Description of this major project
    pub description: Option<String>,
}

pub struct MajorProjectSubmissionEboard {
    /// Username of member who submitted this major project
    pub uid: String,
    /// Name of this major project
    pub name: String,
    /// Description of this major project
    pub description: Option<String>,
    /// idk something fs
    pub status: MajorProjectStatus,
}

pub struct CoopSubmission {
    pub uid: String,
    pub date: NaiveDateTime,
    pub semester: CoopSemester,
}

pub struct IntroFormSubmission {
    pub uid: String,
    pub social_events: String,
    pub comments: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct BatchConditionSubmission {
    pub value: i32,
    pub condition: BatchConditionType,
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
    pub member_users: Vec<MemberBatchUser>,
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

pub struct Batch {
    pub name: String,
    pub creator: String,
    pub conditions: Vec<FreshmanBatchSubmission>,
    pub freshmen: Vec<i32>,
    pub members: Vec<String>,
}
