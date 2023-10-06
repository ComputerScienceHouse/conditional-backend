// TODO: Define API Schema that the API routes will deliver to the frontend
// These are explicitly different from the DB schema. As, for example,
// directorship attendance may be relayed to the fronted as a list of member
// names / usernames, while directorship attendance is stored in the database
// as relations in one of two tables

use chrono::NaiveDateTime;
use sqlx::types::Json;

struct SeminarAttendance {
    name: String,
    date: NaiveDateTime,
    body: Json<String>,
}