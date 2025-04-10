use crate::handlers::listen::listen;
use actix_web::{HttpResponse, Responder, get, web};

#[get("/test/{zipcode}/{max_distance}/{name}/{phone_number}/{email}")]
async fn test(path: web::Path<(String, u16, String, String, String)>) -> impl Responder {
    let (zipcode, max_distance, name, phone_number, email) = path.into_inner();
    match listen(zipcode, max_distance, name, phone_number, email).await {
        Ok(_) => HttpResponse::Ok().body("Started listening for appointments."),
        Err(e) => {
            eprintln!("Failed to start listener: {:?}", e);
            HttpResponse::InternalServerError().body("Failed to start listener.")
        }
    }
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(test);
}
