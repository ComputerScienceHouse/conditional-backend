use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgHasArrayType, PgTypeInfo};
use sqlx::types::chrono;
use sqlx::FromRow;
use utoipa::ToSchema;

// OtherMeeting
#[derive(sqlx::Type, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Copy, Hash)]
#[sqlx(type_name = "meeting_type_enum")]
pub enum MeetingType {
  Seminar,
  Directorship,
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

impl PgHasArrayType for ConditionalStatus {
  fn array_type_info() -> PgTypeInfo {
    PgTypeInfo::with_name("_conditional_status_enum")
  }
}
#[derive(FromRow, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct HouseMeetingAttendance {
  pub uid: i32,
  pub house_meeting_id: i32,
  pub attendance_status: AttendanceStatus,
  pub excuse: String,
}

// IntroEvalData
#[derive(sqlx::Type, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Copy, ToSchema)]
#[sqlx(type_name = "eval_status_enum")]
pub enum EvalStatusEnum {
  Pending,
  Passed,
  Failed,
}

impl PgHasArrayType for EvalStatusEnum {
  fn array_type_info() -> PgTypeInfo {
    PgTypeInfo::with_name("_eval_status_enum")
  }
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
