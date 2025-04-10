use actix_web::{HttpResponse, Responder, get, web};

#[get("/ping")]
async fn ping() -> impl Responder {
    HttpResponse::Ok().body("pong")
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(ping);
}
