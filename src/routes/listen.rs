use actix_web::{HttpResponse, Responder, get, web};
use mongodb::{Client, Collection, options::ClientOptions};
use serde::{Deserialize, Serialize};
use std::env;
use std::error::Error;
use std::fmt;
use tokio::sync::OnceCell;

use crate::handlers::listen::listen;
use crate::models::dmvservice::DMVService;

// --------------------------------------------------------------------------
// DMV Service Definition
// --------------------------------------------------------------------------

// Custom error type for an unknown service title.
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

/// Matches a service title to a DMVService variant.
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

// --------------------------------------------------------------------------
// MongoDB Asynchronous Client Initialization Using tokio::sync::OnceCell
// --------------------------------------------------------------------------

/// The appointment request document to be stored in MongoDB.
#[derive(Debug, Serialize, Deserialize)]
pub struct AppointmentRequest {
    pub zipcode: String,
    pub max_distance: u16,
    pub name: String,
    pub phone_number: String,
    pub email: String,
    pub service_title: String,
    pub selector: String,
    pub dates: Vec<String>,
}

/// Global asynchronous MongoDB client using `OnceCell`.
#[cfg(not(debug_assertions))]
static MONGO_CLIENT: OnceCell<Client> = OnceCell::const_new();

/// Asynchronously get or initialize the global MongoDB client.
#[cfg(not(debug_assertions))]
pub async fn get_mongo_client() -> &'static Client {
    MONGO_CLIENT
        .get_or_init(|| async {
            let uri = env::var("MONGODB_URI").expect("MONGODB_URI must be set");
            let client_options = ClientOptions::parse(&uri)
                .await
                .expect("Failed to parse MongoDB options");
            Client::with_options(client_options).expect("Failed to initialize MongoDB client")
        })
        .await
}

/// Asynchronously obtain the MongoDB collection for appointment requests.
#[cfg(not(debug_assertions))]
pub async fn get_appointment_collection() -> Collection<AppointmentRequest> {
    let client = get_mongo_client().await;
    let db = client.database("InstantDMV");
    db.collection::<AppointmentRequest>("users_nc")
}

// --------------------------------------------------------------------------
// Actix-web Handler and Server Setup
// --------------------------------------------------------------------------

#[get("/test/{zipcode}/{max_distance}/{name}/{phone_number}/{email}/{service_title}/{dates}")]
async fn test(
    path: web::Path<(String, u16, String, String, String, String, String)>,
) -> impl Responder {
    let (zipcode, max_distance, name, phone_number, email, service_title, dates_str) =
        path.into_inner();

    // Parse the comma-separated dates.
    let dates: Vec<String> = dates_str.split(',').map(|s| s.trim().to_string()).collect();

    // Get the DMV service by title.
    let service_type = match get_service_by_title(&service_title) {
        Ok(service) => service,
        Err(e) => {
            eprintln!("Invalid service title: {}", e);
            return HttpResponse::BadRequest().body(format!("Invalid service title: {}", e));
        }
    };

    // Create an appointment request document.
    #[cfg(not(debug_assertions))]
    let new_request = AppointmentRequest {
        zipcode: zipcode.clone(),
        max_distance,
        name: name.clone(),
        phone_number: phone_number.clone(),
        email: email.clone(),
        service_title: service_title.clone(),
        selector: service_type.selector().to_string(),
        dates: dates.clone(),
    };

    // Insert the appointment request into MongoDB asynchronously if in release mode
    #[cfg(not(debug_assertions))]
    {
        let collection = get_appointment_collection().await;
        if let Err(e) = collection.insert_one(new_request).await {
            eprintln!("Failed to insert into MongoDB: {:?}", e);
            return HttpResponse::InternalServerError().body("Failed to store request.");
        }
    }

    // Call the listen function (business logic).
    match listen(
        zipcode,
        max_distance,
        name,
        phone_number,
        email,
        service_type,
        dates,
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

/// Configures the Actix Web application routes.
pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(test);
}
