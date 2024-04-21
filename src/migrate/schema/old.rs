use crate::migrate::schema::new::EvalStatusEnum;
use crate::migrate::schema::new::MajorProjectStatusEnum;
use crate::migrate::schema::new::SemesterEnum;
use serde::{Deserialize, Serialize};
use sqlx::types::chrono;
use sqlx::FromRow;
use utoipa::ToSchema;

use super::new::ConditionalStatus;

// Directorships
#[derive(sqlx::Type, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Copy, ToSchema)]
#[sqlx(type_name = "committees_enum")]
pub enum CommitteeType {
    Evaluations,
    History,
    Social,
    Opcomm,
    #[sqlx(rename = "R&D")]
    Rnd,
    #[sqlx(rename = "House Improvements")]
    Imps,
    Financial,
    /// Unused (?), but valid within the API
    Chairman,
    #[sqlx(rename = "Ad-Hoc")]
    Adhoc,
    #[sqlx(rename = "Public Relations")]
    PR,
}

impl CommitteeType {
    pub fn get_value(&self) -> String {
        match self {
            CommitteeType::Evaluations => "Evaluations".to_string(),
            CommitteeType::History => "History".to_string(),
            CommitteeType::Social => "Social".to_string(),
            CommitteeType::Opcomm => "Opcomm".to_string(),
            CommitteeType::Rnd => "R&D".to_string(),
            CommitteeType::Imps => "House Improvements".to_string(),
            CommitteeType::Financial => "Financial".to_string(),
            CommitteeType::Chairman => "Chairman".to_string(),
            CommitteeType::Adhoc => "Ad-Hoc".to_string(),
            CommitteeType::PR => "Public Relations".to_string(),
        }
    }
}

#[derive(sqlx::Type, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Copy, FromRow)]
pub struct Committee {
    pub id: i32,
    pub committee: CommitteeType,
    pub timestamp: chrono::NaiveDateTime,
    pub active: bool,
    pub approved: bool,
}

// Seminars
#[derive(sqlx::Type, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, FromRow)]
pub struct Seminar {
    pub id: i32,
    pub name: String,
    pub timestamp: chrono::NaiveDateTime,
    pub active: bool,
    pub approved: bool,
}

// FreshmanAccount
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq, FromRow)]
pub struct FreshmanAccount {
    pub id: i32,
    pub name: String,
    pub eval_date: chrono::NaiveDate,
    pub onfloor_status: bool,
    pub room_number: Option<String>,
    pub signatures_missed: Option<i32>,
    pub rit_username: String,
}

// Coops
#[derive(sqlx::Type, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Copy, ToSchema)]
#[sqlx(type_name = "co_op_enum")]
pub enum CoopEnum {
    Fall,
    Spring,
    Neither,
}

impl CoopEnum {
    pub fn get_semester_enum(&self) -> SemesterEnum {
        match self {
            CoopEnum::Fall => SemesterEnum::Fall,
            CoopEnum::Spring => SemesterEnum::Spring,
            CoopEnum::Neither => SemesterEnum::Summer,
        }
    }
}

#[derive(sqlx::Type, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Copy, ToSchema)]
#[sqlx(type_name = "conditional_enum")]
pub enum ConditionalEnum {
    Pending,
    Passed,
    Failed,
}

impl ConditionalEnum {
    pub fn get_conditional_status_enum(&self) -> ConditionalStatus {
        match self {
            ConditionalEnum::Pending => ConditionalStatus::Pending,
            ConditionalEnum::Passed => ConditionalStatus::Passed,
            ConditionalEnum::Failed => ConditionalStatus::Failed,
        }
    }
}

#[derive(FromRow, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Coop {
    pub id: i32,
    pub uid: String,
    pub date_created: chrono::NaiveDate,
    pub semester: CoopEnum,
}

// HouseMeeting
#[derive(FromRow, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct HouseMeeting {
    pub id: i32,
    pub date: chrono::NaiveDate,
}

#[derive(sqlx::Type, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Copy, ToSchema)]
#[sqlx(type_name = "attendance_enum")]
pub enum AttendanceStatus {
    Attended,
    Excused,
    Absent,
}

impl AttendanceStatus {
    pub fn get_new_value(&self) -> crate::migrate::schema::new::AttendanceStatus {
        match self {
            AttendanceStatus::Attended => crate::migrate::schema::new::AttendanceStatus::Attended,
            AttendanceStatus::Excused => crate::migrate::schema::new::AttendanceStatus::Excused,
            AttendanceStatus::Absent => crate::migrate::schema::new::AttendanceStatus::Absent,
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq, FromRow)]
pub struct MemberHouseMeetingAttendance {
    pub id: i32,
    pub uid: String,
    pub meeting_id: i32,
    pub excuse: Option<String>,
    pub attendance_status: AttendanceStatus,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq, FromRow)]
pub struct FreshmanHouseMeetingAttendance {
    pub id: i32,
    pub fid: i32,
    pub meeting_id: i32,
    pub excuse: Option<String>,
    pub attendance_status: AttendanceStatus,
}

// FreshmenEvalData
#[derive(sqlx::Type, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Copy, ToSchema)]
#[sqlx(type_name = "freshman_eval_enum")]
pub enum FreshmanEvalEnum {
    Pending,
    Passed,
    Failed,
}

#[derive(sqlx::Type, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Copy, ToSchema)]
#[sqlx(type_name = "freshman_project_enum")]
pub enum FreshmanProjectEnum {
    Pending,
    Passed,
    Failed,
}

impl FreshmanEvalEnum {
    pub fn get_eval_enum(&self) -> EvalStatusEnum {
        match self {
            FreshmanEvalEnum::Pending => EvalStatusEnum::Pending,
            FreshmanEvalEnum::Passed => EvalStatusEnum::Passed,
            FreshmanEvalEnum::Failed => EvalStatusEnum::Failed,
        }
    }
}

#[derive(FromRow, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct FreshmanEvalData {
    pub id: i32,
    pub uid: String,
    pub freshman_project: Option<FreshmanProjectEnum>,
    pub eval_date: chrono::NaiveDateTime,
    pub signatures_missed: i32,
    pub social_events: Option<String>,
    pub other_notes: Option<String>,
    pub freshman_eval_result: FreshmanEvalEnum,
    pub active: Option<bool>,
}

#[derive(sqlx::Type, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Copy, ToSchema)]
#[sqlx(type_name = "major_project_enum")]
pub enum MajorProjectEnum {
    Pending,
    Passed,
    Failed,
}

impl MajorProjectEnum {
    pub fn get_new_enum(&self) -> MajorProjectStatusEnum {
        match self {
            MajorProjectEnum::Pending => MajorProjectStatusEnum::Pending,
            MajorProjectEnum::Passed => MajorProjectStatusEnum::Passed,
            MajorProjectEnum::Failed => MajorProjectStatusEnum::Failed,
        }
    }
}

#[derive(FromRow, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct MajorProject {
    pub id: i32,
    pub uid: String,
    pub name: String,
    pub description: Option<String>,
    pub active: bool,
    pub status: MajorProjectEnum,
    pub date: chrono::NaiveDate,
}

#[derive(sqlx::Type, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Copy, ToSchema)]
#[sqlx(type_name = "spring_eval_enum")]
pub enum SpringEvalEnum {
    Pending,
    Passed,
    Failed,
}

impl SpringEvalEnum {
    pub fn get_new_enum(&self) -> EvalStatusEnum {
        match self {
            SpringEvalEnum::Pending => EvalStatusEnum::Pending,
            SpringEvalEnum::Passed => EvalStatusEnum::Passed,
            SpringEvalEnum::Failed => EvalStatusEnum::Failed,
        }
    }
}
#[derive(FromRow, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct SpringEval {
    pub id: i32,
    pub uid: String,
    pub active: bool,
    pub date_created: chrono::NaiveDate,
    pub status: SpringEvalEnum,
}

#[derive(FromRow, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct FCA {
    pub id: i32,
    pub fid: i32,
    pub meeting_id: i32,
}

#[derive(FromRow, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct FSA {
    pub id: i32,
    pub fid: i32,
    pub seminar_id: i32,
}

#[derive(FromRow, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct MCA {
    pub id: i32,
    pub uid: String,
    pub meeting_id: i32,
}

#[derive(FromRow, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct MSA {
    pub id: i32,
    pub uid: String,
    pub seminar_id: i32,
}

#[derive(FromRow, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Conditional {
    pub id: i32,
    pub uid: String,
    pub description: String,
    pub date_created: chrono::NaiveDate,
    pub date_due: chrono::NaiveDate,
    pub active: bool,
    pub status: ConditionalEnum,
    pub i_evaluation: Option<i32>,
    pub s_evaluation: Option<i32>,
}
