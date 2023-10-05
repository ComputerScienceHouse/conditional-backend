use actix_cors::Cors;
use actix_web::{http, App, HttpServer};
use std::env;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin(&env::var("DOMAIN").unwrap_or("localhost".to_string()))
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"]);
        App::new()
            .wrap(cors)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
