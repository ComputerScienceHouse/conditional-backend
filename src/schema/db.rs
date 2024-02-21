use derive_more::{Deref, Display};
use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgHasArrayType, PgTypeInfo};
use sqlx::types::chrono;
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(sqlx::Type, Serialize, Deserialize, Clone, Display, Deref, Debug)]
pub struct ID {
    pub id: i32,
}

// OtherMeeting
#[derive(sqlx::Type, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Copy, Hash, ToSchema)]
#[sqlx(type_name = "meeting_type_enum")]
pub enum MeetingType {
    Seminar,
    Directorship,
}

#[derive(FromRow, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct BatchCondition {
    pub id: i32,
    pub batch_id: i32,
    pub value: i32,
    pub criterion: BatchCriterion,
    pub comparison: BatchComparison,
}

#[derive(FromRow, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct OtherMeeting {
    pub id: i32,
    pub datetime: chrono::NaiveDateTime,
    pub name: String,
    pub meeting_type: MeetingType,
    pub approved: bool,
}

// User
#[derive(FromRow, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub intro_id: Option<String>,
    pub ipa_unique_id: Option<String>,
    pub rit_username: String,
    pub csh_username: Option<String>,
    pub is_csh: bool,
    pub is_intro: bool,
}

// Coop
#[derive(sqlx::Type, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Copy, ToSchema)]
#[sqlx(type_name = "semester_enum")]
pub enum SemesterEnum {
    Fall,
    Spring,
    Summer,
}

impl PgHasArrayType for SemesterEnum {
    fn array_type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("_semester_enum")
    }
}

#[derive(FromRow, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Coop {
    pub id: i32,
    pub uid: i32,
    pub date_created: chrono::NaiveDate,
    pub semester: SemesterEnum,
}

// HouseMeeting
#[derive(FromRow, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct HouseMeeting {
    pub id: i32,
    pub date: chrono::NaiveDate,
}

#[derive(sqlx::Type, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Copy, ToSchema)]
struct Absences {
    uid: i32,
    count: Option<i64>,
}

#[derive(sqlx::Type, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Copy, ToSchema)]
struct DateWrapper {
    date: chrono::NaiveDate,
}

#[derive(sqlx::Type, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, ToSchema)]
struct AbsenceWrapper {
    date: chrono::NaiveDate,
    excuse: Option<String>,
}

#[derive(sqlx::Type, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Copy, ToSchema)]
#[sqlx(type_name = "hm_attendance_status_enum")]
pub enum AttendanceStatus {
    Attended,
    Excused,
    Absent,
}

impl PgHasArrayType for AttendanceStatus {
    fn array_type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("_hm_attendance_status_enum")
    }
}

#[derive(sqlx::Type, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Copy, ToSchema)]
#[sqlx(type_name = "conditional_status_enum")]
pub enum ConditionalStatus {
    Pending,
    Passed,
    Failed,
}

#[derive(FromRow, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct HouseMeetingAttendance {
    pub uid: i32,
    pub house_meeting_id: i32,
    pub attendance_status: AttendanceStatus,
    pub excuse: String,
}

#[derive(sqlx::Type, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Copy, ToSchema)]
pub enum BatchCriterion {
    Seminar,
    Directorship,
    Packet,
    #[sqlx(rename = "Missed_HM")]
    MissedHM,
}

#[derive(sqlx::Type, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Copy, ToSchema)]
pub enum BatchComparison {
    Greater,
    Equal,
    Less,
}

// IntroEvalData
#[derive(sqlx::Type, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Copy, ToSchema)]
#[sqlx(type_name = "eval_status_enum")]
pub enum EvalStatusEnum {
    Pending,
    Passed,
    Failed,
}

#[derive(sqlx::Type, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Copy, ToSchema)]
#[sqlx(type_name = "major_project_status_enum")]
pub enum MajorProjectStatusEnum {
    Pending,
    Passed,
    Failed,
}

impl PgHasArrayType for MajorProjectStatusEnum {
    fn array_type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("_major_project_status_enum")
    }
}
