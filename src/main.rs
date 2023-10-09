use actix_cors::Cors;
use actix_web::{App, HttpServer};
use conditional_backend::app::{configure_app, get_app_data};
use dotenv::dotenv;
use lazy_static::lazy_static;
use std::env;

lazy_static! {
    static ref SECURITY_ENABLED: bool = env::var("SECURITY_ENABLED")
        .map(|x| x.parse::<bool>().unwrap_or(true))
        .unwrap_or(true);
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();
    let app_data = get_app_data().await;
    HttpServer::new(move || {
        let cors = if *SECURITY_ENABLED {
            Cors::default()
        } else {
            Cors::permissive()
        }
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
