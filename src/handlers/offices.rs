use crate::cache::OFFICE_CACHE;
use crate::models::offices::OfficeAvailability;

pub async fn get_available_appointments()
-> Result<Vec<OfficeAvailability>, Box<dyn std::error::Error>> {
    let offices: Vec<_> = OFFICE_CACHE.iter().map(|entry| entry.1.clone()).collect();
    Ok(offices)
}
