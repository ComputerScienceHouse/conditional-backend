use actix_cors::Cors;
use actix_web::{App, HttpServer};
use std::env;

mod app;
use app::get_app_data;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let app_data = get_app_data().await;
    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin(&env::var("DOMAIN").unwrap_or("localhost".to_string()))
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"]);
        App::new().wrap(cors).app_data(app_data.clone())
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
