pub mod app;

/// Data structure for API routes and DB access
pub mod schema {
    /// Schema returned and consumed by the API
    pub mod api;
    /// Schema retrieved from and written to the DB
    pub mod db;
}

pub mod api;

pub mod ldap;

pub mod auth;
pub mod auth_service;

pub mod migrate;
