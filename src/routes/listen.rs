use crate::handlers::listen::listen;
use actix_web::{HttpResponse, Responder, get, web};

#[get("/test/{zipcode}/{max_distance}")]
async fn offices(path: web::Path<(String, u16)>) -> impl Responder {
    let (zipcode, max_distance) = path.into_inner();

    match listen(zipcode, max_distance).await {
        Ok(_) => HttpResponse::Ok().body("Started listening for appointments."),
        Err(e) => {
            eprintln!("Failed to start listener: {:?}", e);
            HttpResponse::InternalServerError().body("Failed to start listener.")
        }
    }
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(offices);
}
