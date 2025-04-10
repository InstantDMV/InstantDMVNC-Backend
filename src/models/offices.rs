use chrono::NaiveDate;
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct OfficeAvailability {
    pub is_reservable: bool,
    pub office_name: String,
    pub street_address: String,
    pub distance: u16,
    pub zip_code: String,
    pub available_dates: Vec<NaiveDate>,
    pub selected_date: Option<NaiveDate>,
}
