// TODO: Define API Schema that the API routes will deliver to the frontend
// These are explicitly different from the DB schema. As, for example,
// directorship attendance may be relayed to the fronted as a list of member
// names / usernames, while directorship attendance is stored in the database
// as relations in one of two tables

use chrono::{NaiveDate, NaiveDateTime};
use sqlx::types::Json;

use super::db::{AttendanceStatus, MajorProjectStatus};

struct MeetingAttendance {
    name: String,
    date: NaiveDateTime,
    body: Json<String>,
}

struct IndividualHouseAttendance {
    name: String,
    att_status: AttendanceStatus,
}

struct HouseAttendance {
    date: NaiveDateTime,
    body: Json<IndividualHouseAttendance>,
}

pub struct MajorProjectSubmission {
    /// Unique id for this major project
    pub id: i32,
    /// Username of member who submitted this major project
    pub uid: String,
    /// Name of this major project
    pub name: String,
    /// Description of this major project
    pub description: Option<String>,
}

pub struct MajorProjectSubmissionEboard {
    /// Unique id for this major project
    pub id: i32,
    /// Username of member who submitted this major project
    pub uid: String,
    /// Name of this major project
    pub name: String,
    /// Description of this major project
    pub description: Option<String>,
    /// idk something fs
    pub status: MajorProjectStatus,
}
