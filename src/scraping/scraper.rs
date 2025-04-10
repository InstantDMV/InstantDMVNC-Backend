extern crate country;
extern crate postal_code;

use country::Country;
use postal_code::PostalCode;

use crate::models::offices::OfficeAvailability;
use crate::scraping::constants::*;
use anyhow::Result;
use chrono::{Datelike, Local, NaiveDate};
use regex::Regex;
use std::sync::Arc;
use std::time::Duration;
use thirtyfour::prelude::*;
use thirtyfour::support::sleep;
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::{error, info};

pub struct NCDMVScraper {
    name: String,
    phone_number: String,
    email: String,
    zipcode: String,
    max_distance: u16,
}

impl NCDMVScraper {
    pub async fn new(
        zipcode: String,
        max_distance: u16,
        name: String,
        phone_number: String,
        email: String,
    ) -> Result<Self> {
        if Self::validate(&zipcode).await? {
            Ok(NCDMVScraper {
                name,
                phone_number,
                email,
                zipcode,
                max_distance,
            })
        } else {
            Err(anyhow::anyhow!("Invalid ZIP code"))
        }
    }

    async fn validate(zip_code: &String) -> Result<bool> {
        match PostalCode::new(Country::USA, zip_code) {
            Ok(code) => Ok(code.country() == &Country::USA),
            Err(_) => Ok(false),
        }
    }

    pub async fn stream_available_appointments(
        self: Arc<Self>,
        zip_code: String,
        refresh_interval_secs: u64,
        tx: mpsc::Sender<Vec<OfficeAvailability>>,
    ) -> WebDriverResult<()> {
        let mut caps = DesiredCapabilities::chrome();
        // _ = caps.set_headless(); //for debugging comment this line
        let driver = WebDriver::new("http://localhost:60103", caps).await?;
        let driver = Arc::new(driver);

        driver
            .set_window_rect(0, 0, WINDOW_WIDTH, WINDOW_HEIGHT)
            .await?;

        driver.goto(BASE_URL).await?;

        // Initial navigation
        driver
            .find(By::Id(BUTTON_MAKE_APPT_ID))
            .await?
            .click()
            .await?;

        // Wait for navigation
        sleep(Duration::from_secs(8)).await;

        driver
            .find(By::Css(SELECTOR_SECOND_FORM_CHILD))
            .await?
            .click()
            .await?;

        sleep(Duration::from_secs(5)).await;

        let search_input = driver.find(By::Id(SEARCH_INPUT_ID)).await?;
        search_input.click().await?;
        search_input.send_keys(&zip_code).await?;

        sleep(Duration::from_secs(3)).await;

        driver
            .find(By::Css(INPUT_RESULTS_SELECTOR))
            .await?
            .click()
            .await?;

        // Now that we're on the results page, start checking periodically
        let mut refresh_interval = interval(Duration::from_secs(refresh_interval_secs));

        loop {
            refresh_interval.tick().await;
            let scraper = Arc::clone(&self);

            match scraper.scrape_and_check_available_dates(&driver).await {
                Ok(results) => {
                    if tx.send(results).await.is_err() {
                        // Channel closed, receiver dropped
                        break;
                    }
                }
                Err(e) => {
                    error!("Error scraping page: {:?}", e);
                }
            }

            // Refresh the page for new data
            if let Err(e) = driver.refresh().await {
                error!("Failed to refresh page: {:?}", e);
                break;
            }

            // Wait for page to stabilize after refresh
            sleep(Duration::from_secs(3)).await;
        }

        Ok(())
    }

    async fn scrape_and_check_available_dates(
        self: Arc<Self>,
        driver: &WebDriver,
    ) -> WebDriverResult<Vec<OfficeAvailability>> {
        let mut results = Vec::new();

        // Find all office elements
        let office_elements = driver
            .find_all(By::Css(&format!(".{}", DMV_ITEM_CLASS)))
            .await?;

        for office_el in office_elements {
            // Get office classes to check if reservable
            let classes = office_el.class_name().await?.unwrap_or_default();
            let is_reservable = classes.contains(ACTIVE_UNIT_CLASS);

            // Get the office name
            let office_divs = office_el.find_all(By::Tag("div")).await?;
            let mut office_name = String::new();
            if office_divs.len() > 1 {
                office_name = office_divs[1].text().await?.trim().to_string();
            }

            // Get the address
            let addr_el = office_el
                .find(By::Css(&format!(".{}", DMV_CHILD_CLASS)))
                .await?;
            let addr = addr_el.text().await?.trim().to_string();

            // Extract zip code from address
            let zip_regex = Regex::new(r"\b\d{5}\b").unwrap();
            let zip_code = zip_regex
                .find(&addr)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();

            // Extract street address
            let street_address = addr
                .replace(&zip_code, "")
                .trim()
                .trim_end_matches(',')
                .to_string();

            let distance: u16 = match office_divs.iter().rev().next() {
                Some(div) => {
                    let text = div
                        .text()
                        .await
                        .unwrap_or_default()
                        .replace(" Miles", "")
                        .replace("text=", "");
                    let parsed_distance =
                        text.parse::<f32>().expect("failed to parse the distance");
                    parsed_distance.round() as u16
                }
                None => 0,
            };

            if distance > self.max_distance {
                continue;
            }
            let mut office_availability = OfficeAvailability {
                is_reservable,
                office_name,
                street_address,
                zip_code,
                distance,
                available_dates: Vec::new(),
                selected_date: None,
            };

            // If office is reservable, try to click on it and check for available dates
            if is_reservable {
                if let Ok(_) = office_el.click().await {
                    // Wait for calendar to load
                    sleep(Duration::from_secs(3)).await;

                    // Get the month shown in the calendar
                    let month_text = match driver.find(By::Css(".ui-datepicker-month")).await {
                        Ok(el) => match el.text().await {
                            Ok(text) => text,
                            Err(_) => String::new(),
                        },
                        Err(_) => String::new(),
                    };

                    // Get year shown in the calendar
                    let year_text = match driver.find(By::Css(".ui-datepicker-year")).await {
                        Ok(el) => match el.text().await {
                            Ok(text) => text,
                            Err(_) => String::new(),
                        },
                        Err(_) => String::new(),
                    };

                    let year = year_text
                        .parse::<i32>()
                        .unwrap_or_else(|_| Local::now().year());

                    // Find all available dates (with the active class)
                    if let Ok(date_elements) = driver
                        .find_all(By::Css(&format!(
                            "a.{}",
                            AVAILABLE_DATE_CLASS.replace(" ", ".")
                        )))
                        .await
                    {
                        for date_el in date_elements {
                            if let Ok(day_text) = date_el.text().await {
                                if let Ok(day) = day_text.parse::<u32>() {
                                    // Convert month name to month number (1-12)
                                    let month = match month_text.as_str() {
                                        "January" => 1,
                                        "February" => 2,
                                        "March" => 3,
                                        "April" => 4,
                                        "May" => 5,
                                        "June" => 6,
                                        "July" => 7,
                                        "August" => 8,
                                        "September" => 9,
                                        "October" => 10,
                                        "November" => 11,
                                        "December" => 12,
                                        _ => continue, //wtf this shouldnt happen unless the gods reinvent the universe as we know it
                                    };

                                    if let Some(date) = NaiveDate::from_ymd_opt(year, month, day) {
                                        office_availability.available_dates.push(date);
                                    }
                                }
                            }
                        }
                    }

                    // If we have available dates, try to select the earliest one
                    if !office_availability.available_dates.is_empty() {
                        // Sort dates in ascending order
                        let mut sorted_dates = office_availability.available_dates.clone();
                        sorted_dates.sort();

                        if let Some(earliest_date) = sorted_dates.first() {
                            // Find the day number as text
                            let day_text = earliest_date.day().to_string();

                            // Try to click on that date
                            if let Ok(_) = driver.find(By::LinkText(&day_text)).await {
                                info!(
                                    "Selected date {} for office {}",
                                    earliest_date, office_availability.office_name
                                );
                            }
                        }
                    }
                    if driver.find(By::Tag("body")).await.expect("failed to read pages text").text().await.unwrap().contains("This office does not currently have any appointments available in the next 90 days. Please try scheduling an appointment at another office or try again tomorrow when a new day's appointments will be available.") {
                        // Go back to the list view for next office
                        if let Ok(back_button) = driver.find(By::Id("BackButton")).await {
                            let _ = back_button.click().await;
                            sleep(Duration::from_secs(1)).await;
                        }
                        break;

                    }

                    // next button for inputting info
                    if let Ok(back_button) = driver.find(By::ClassName("next-button")).await {
                        let _ = back_button.click().await;
                        sleep(Duration::from_secs(1)).await;
                    }

                    if driver
                        .find(By::Tag("body"))
                        .await
                        .expect("failed to read pages text")
                        .text()
                        .await
                        .unwrap()
                        .contains("Please select a date and time to continue.")
                    {
                        // Go back to the list view for next office
                        if let Ok(back_button) = driver.find(By::Id("BackButton")).await {
                            let _ = back_button.click().await;
                            sleep(Duration::from_secs(1)).await;
                        }
                        break;
                    }

                    let name_clone = self.name.clone();
                    let names: Vec<&str> = name_clone.split('_').collect();
                    let fname = names[0];
                    let lname = if names.len() > 1 { names[1] } else { "" };

                    let _ = driver
                        .find(By::Id(FNAME_INPUT_ID))
                        .await
                        .unwrap()
                        .send_keys(fname);

                    let _ = driver
                        .find(By::Id(LNAME_INPUT_ID))
                        .await
                        .unwrap()
                        .send_keys(lname);

                    let _ = driver
                        .find(By::Id(PHONE_NUM_INPUT_ID))
                        .await
                        .unwrap()
                        .send_keys(self.phone_number.as_str());

                    let _ = driver
                        .find(By::Id(EMAIL_INPUT_ID))
                        .await
                        .unwrap()
                        .send_keys(self.email.as_str());

                    let _ = driver
                        .find(By::Id(CONFIRM_EMAIL_INPUT_ID))
                        .await
                        .unwrap()
                        .send_keys(self.email.as_str());

                    sleep(Duration::from_secs(2)).await;

                    // next button for inputting info
                    if let Ok(back_button) = driver.find(By::ClassName("next-button")).await {
                        let _ = back_button.click().await;
                        sleep(Duration::from_secs(1)).await;
                    }
                }
            }

            results.push(office_availability);
        }

        Ok(results)
    }

    pub async fn start_appointment_stream(
        self: Arc<Self>,
        refresh_interval_secs: u64,
    ) -> mpsc::Receiver<Vec<OfficeAvailability>> {
        let (tx, rx) = mpsc::channel(117); // Buffer size of 117 for 117 dmvs in NC
        info!("scraping nc dmv data with date checking");

        // Clone shared state to move into the task
        let scraper = Arc::clone(&self);
        let zip_code = scraper.zipcode.clone();

        tokio::spawn(async move {
            if let Err(e) = scraper
                .stream_available_appointments(zip_code, refresh_interval_secs, tx)
                .await
            {
                error!("Error starting appointment stream: {:?}", e);
            }
        });

        rx
    }
}
