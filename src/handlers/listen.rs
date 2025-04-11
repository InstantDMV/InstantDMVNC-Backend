use crate::cache::OFFICE_CACHE;
use crate::models::dmvservice::DMVService;
use crate::scraping::scraper::NCDMVScraper;
use anyhow::Result;
use std::sync::Arc;
use tokio::task;

pub async fn listen(
    zipcode: String,
    max_distance: u16,
    name: String,
    phone_number: String,
    email: String,
    service_type: DMVService,
    dates: Vec<String>,
) -> Result<()> {
    task::spawn(async move {
        match NCDMVScraper::new(zipcode.clone(), max_distance, name, phone_number, email).await {
            Ok(scraper) => {
                let scraper = Arc::new(scraper);
                let mut receiver = scraper
                    .clone()
                    .start_appointment_stream(1, service_type, dates)
                    .await;

                while let Some(offices) = receiver.recv().await {
                    for office in offices {
                        OFFICE_CACHE
                            .insert(office.office_name.clone(), office)
                            .await;
                    }
                }

                tracing::warn!("Receiver closed for {}", zipcode);
            }
            Err(e) => {
                tracing::error!("Failed to start scraper for {}: {:?}", zipcode, e);
            }
        }
    });

    Ok(())
}
