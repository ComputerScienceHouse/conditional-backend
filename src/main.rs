use actix_cors::Cors;
use actix_web::{App, HttpServer};
use conditional_backend::{
    app::{configure_app, get_app_data},
    ldap::*,
};
use dotenv::dotenv;
use log::{log, Level};
use std::env;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();
    let app_data = get_app_data().await;
    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin(&env::var("DOMAIN").unwrap_or("localhost".to_string()))
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"]);
        App::new()
            .wrap(cors)
            .configure(configure_app)
            .app_data(app_data.clone())
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
