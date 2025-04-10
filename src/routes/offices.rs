use crate::handlers::offices::get_available_appointments;
use actix_web::{HttpResponse, Responder, get, web};

#[get("/all")]
async fn offices() -> impl Responder {
    match get_available_appointments().await {
        Ok(data) => HttpResponse::Ok().json(data),
        Err(e) => HttpResponse::InternalServerError().body(format!("Error: {}", e)),
    }
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(offices);
}
