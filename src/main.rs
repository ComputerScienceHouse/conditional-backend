use actix_cors::Cors;
use actix_web::http::header;
use actix_web::{middleware::Logger, App, HttpServer};
use conditional_backend::app::{configure_app, get_app_data};
use dotenv::dotenv;
use lazy_static::lazy_static;
use log::{info, warn};
use std::env;

lazy_static! {
    static ref SECURITY_ENABLED: bool = env::var("SECURITY_ENABLED")
        .map(|x| x.parse::<bool>().unwrap_or(true))
        .unwrap_or(true);
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // parse CLI args for migration
    let args: Vec<String> = env::args().collect();
    if let Some(arg) = args.get(1) {
        if arg == "remigrate" {
            todo!("remigrate");
        } else if arg == "sync" {
            println!("sync!");
            return Ok(());
        }
    }

    dotenv().ok();
    env_logger::init();
    if *SECURITY_ENABLED {
        info!(
            "Starting with security enabled. If in development, it is recommended you disable \
             this."
        )
    } else {
        warn!("Starting with security disabled. THIS SHOULD NOT BE USED IN PRODUCTION.")
    }
    let app_data = get_app_data().await;
    HttpServer::new(move || {
        let _cors = if *SECURITY_ENABLED {
            Cors::default()
                .allow_any_origin()
                // .allowed_origin(&env::var("DOMAIN").unwrap_or("localhost".to_string()))
                .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
                .supports_credentials()
                .allowed_headers(vec![
                    header::AUTHORIZATION,
                    header::ACCEPT,
                    header::CONTENT_TYPE,
                ])
        } else {
            Cors::permissive()
        };
        let cors = Cors::permissive();
        App::new()
            // .wrap(CSHAuth::enabled())
            .wrap(cors)
            .wrap(Logger::new(
                "%a \"%r\" %s %b \"%{Referer}i\" \"%{User-Agent}i\" %T",
            ))
            .configure(configure_app)
            .app_data(app_data.clone())
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
