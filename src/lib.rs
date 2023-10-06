pub mod app;

/// Data structure for API routes and DB access
pub mod schema {
    /// Schema returned and consumed by the API
    pub mod api;
    /// Schema retrieved from and written to the DB
    pub mod db;
}
pub mod api {
    pub mod attendance {
        pub mod routes;
    }

    pub mod forms {
        pub mod routes;
    }
}

/// Utility functions
pub mod util;

pub mod models;

// Internal routes to access the conditional database, separated by table
// pub mod table;
