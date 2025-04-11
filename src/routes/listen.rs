use crate::handlers::listen::listen;
use crate::models::dmvservice::DMVService;
use actix_web::{HttpResponse, Responder, get, web};
use std::error::Error;
use std::fmt;

// Custom error type for service not found
#[derive(Debug)]
pub struct ServiceNotFoundError {
    title: String,
}

impl fmt::Display for ServiceNotFoundError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Service with title '{}' not found", self.title)
    }
}

impl Error for ServiceNotFoundError {}

// Function to get service type from title
fn get_service_by_title(title: &str) -> Result<DMVService, ServiceNotFoundError> {
    match title {
        "Driver License - First Time" => Ok(DMVService::FirstTime {
            title: "Driver License - First Time",
            selector: "New driver over 18",
        }),
        "Driver License Duplicate" => Ok(DMVService::Duplicate {
            title: "Driver License Duplicate",
            selector: "Replace lost or stolen license",
        }),
        "Driver License Renewal" => Ok(DMVService::Renewal {
            title: "Driver License Renewal",
            selector: "Renew an existing license",
        }),
        "Fees" => Ok(DMVService::Fees {
            title: "Fees",
            selector: "License reinstatement appointment",
        }),
        "ID Card" => Ok(DMVService::IdCard {
            title: "ID Card",
            selector: "State ID card",
        }),
        "Knowledge/Computer Test" => Ok(DMVService::KnowledgeTest {
            title: "Knowledge/Computer Test",
            selector: "Written, traffic signs",
        }),
        "Legal Presence" => Ok(DMVService::LegalPresence {
            title: "Legal Presence",
            selector: "For non-citizens to prove",
        }),
        "Motorcycle Skills Test" => Ok(DMVService::MotorcycleTest {
            title: "Motorcycle Skills Test",
            selector: "Schedule a motorcycle driving skills test",
        }),
        "Non-CDL Road Test" => Ok(DMVService::NonCdlRoadTest {
            title: "Non-CDL Road Test",
            selector: "Schedule a driving skills test",
        }),
        "Permits" => Ok(DMVService::Permits {
            title: "Permits",
            selector: "Adult permit",
        }),
        "Teen Driver Level 1" => Ok(DMVService::TeenDriverLevel1 {
            title: "Teen Driver Level 1",
            selector: "Limited learner permit",
        }),
        "Teen Driver Level 2" => Ok(DMVService::TeenDriverLevel2 {
            title: "Teen Driver Level 2",
            selector: "Limited provisional license",
        }),
        "Teen Driver Level 3" => Ok(DMVService::TeenDriverLevel3 {
            title: "Teen Driver Level 3",
            selector: "Full provisional license",
        }),
        _ => Err(ServiceNotFoundError {
            title: title.to_string(),
        }),
    }
}

#[get("/test/{zipcode}/{max_distance}/{name}/{phone_number}/{email}/{service_title}")]
async fn test(path: web::Path<(String, u16, String, String, String, String)>) -> impl Responder {
    let (zipcode, max_distance, name, phone_number, email, service_title) = path.into_inner();

    // Get service type from title
    let service_type = match get_service_by_title(&service_title) {
        Ok(service) => service,
        Err(e) => {
            eprintln!("Invalid service title: {}", e);
            return HttpResponse::BadRequest().body(format!("Invalid service title: {}", e));
        }
    };

    match listen(
        zipcode,
        max_distance,
        name,
        phone_number,
        email,
        service_type,
    )
    .await
    {
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
