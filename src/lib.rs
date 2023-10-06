/// Data structure for API routes and DB access
pub mod schema {
    /// Schema returned and consumed by the API
    pub mod api;
    /// Schema retrieved from and written to the DB
    pub mod db;
}

//// Internal routes to access the conditional database, separated by table
// pub mod table;
