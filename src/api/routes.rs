use actix_web::{get, HttpResponse, Responder};
#[get("/housing")]
pub async fn get_intro_evals() -> impl Responder {
    HttpResponse::ImATeapot()
}
