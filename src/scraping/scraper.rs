use crate::models::dmvservice::DMVService;
use crate::models::email::RegisterRequest;
use crate::models::offices::OfficeAvailability;
use crate::models::zipcode;
use crate::scraping::constants::*;
use anyhow::Result;
use captcha_oxide::CaptchaSolver;
use captcha_oxide::CaptchaTask;
use captcha_oxide::captcha_types::recaptcha::RecaptchaV2;
use chrono::{Datelike, Local, NaiveDate};
use country::Country;
use dotenv::dotenv;
use once_cell::sync::Lazy;
use postal_code::PostalCode;
use regex::Regex;
use reqwest::Client;
use serde_json::Value;
use serde_json::json;
use std::error::Error;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use tempfile::tempdir;
use thirtyfour::extensions::cdp::ChromeCommand;
use thirtyfour::prelude::*;
use thirtyfour::support::sleep;
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::{error, info};
use uuid::Uuid;
/**
NC DMV has a bug where when an appointment is in the proccess of
being booked it shows as blue (possible to be booked) when really it is taken,
the client has just not finished the form and submitted

this tracks those so we dont miss anything
*/

static FALSLEY_ENABLED_LOCATIONS: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(vec![]));

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
        selector: String,
        dates: Vec<String>,
    ) -> WebDriverResult<()> {
        let mut caps = DesiredCapabilities::chrome();
        let tmp_dir = tempdir()?;

        //bc we run in a vm these help for optimization
        caps.add_arg("--headless")?;
        caps.add_arg("--no-first-run")?;
        caps.add_arg("--disable-popup-blocking")?;
        caps.add_arg("--disable-default-apps")?;
        caps.add_arg("--disable-sync")?;
        caps.add_arg("--remote-debugging-port=0")?;
        caps.add_arg("--disable-gpu")?;
        caps.add_arg("--no-sandbox")?;
        caps.add_arg("--disable-dev-shm-usage")?;
        caps.add_arg("--use-fake-ui-for-media-stream")?;
        caps.add_arg("--use-fake-device-for-media-stream")?;

        caps.add_arg(format!("--user-data-dir={}", tmp_dir.path().display()).as_str())?;

        let user_data_dir = format!("/tmp/chrome-user-data-{}", Uuid::new_v4());
        caps.add_arg(&format!("--user-data-dir={}", user_data_dir))?;

        let driver = WebDriver::new("http://localhost:60103", caps).await?;
        let driver = Arc::new(driver);

        let zipcode_data = zipcode::load_zipcode_data("./zipcodetolatlong.csv");

        info!("{}", zip_code);

        let coordinates = zipcode_data.get(&zip_code).unwrap();

        let latitude = coordinates.0;
        let longitude = coordinates.1;

        driver.goto(BASE_URL).await?;

        let grant_command = ChromeCommand::ExecuteCdpCommand(
            "Browser.grantPermissions".to_string(),
            json!({
                "permissions": ["geolocation"],
                "origin": "https://skiptheline.ncdot.gov"
            }),
        );

        driver.cmd(grant_command).await?;

        // Send Chrome DevTools Protocol command to override geolocation
        let spoof_location_command = ChromeCommand::ExecuteCdpCommand(
            "Page.setGeolocationOverride".to_string(),
            json!({
                "latitude": latitude,
                "longitude": longitude,
                "accuracy": 100.0
            }),
        );

        driver.cmd(spoof_location_command).await?;

        // Initial navigation
        driver
            .find(By::Id(BUTTON_MAKE_APPT_ID))
            .await?
            .click()
            .await?;

        sleep(Duration::from_secs(10)).await;

        // Wait for navigation
        'outer: loop {
            let elements = driver.find_all(By::Css("div.form-control-child")).await?;
            for elem in elements {
                if elem.text().await?.contains(&selector) {
                    if elem.is_clickable().await? {
                        elem.click().await?;
                        break 'outer;
                    }
                }
            }
        }

        sleep(Duration::from_secs(10)).await;

        // Now that we're on the results page, start checking periodically
        let mut refresh_interval = interval(Duration::from_secs(refresh_interval_secs));

        sleep(Duration::from_secs(1)).await;

        loop {
            refresh_interval.tick().await;

            let scraper = Arc::clone(&self);

            match scraper
                .scrape_and_check_available_dates(&driver, &dates)
                .await
            {
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
            sleep(Duration::from_secs(1)).await;
        }

        info!("Quitting Chrome session");

        if let Ok(driver_inner) = Arc::try_unwrap(driver) {
            driver_inner.quit().await?;
        } else {
            error!("Driver Arc still has other references — cannot quit cleanly");
        }

        Ok(())
    }

    async fn scrape_and_check_available_dates(
        self: Arc<Self>,
        driver: &WebDriver,
        dates: &Vec<String>,
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

            if FALSLEY_ENABLED_LOCATIONS
                .lock()
                .unwrap()
                .contains(&office_availability.office_name)
                && is_reservable
            {
                info!("{:?}", *FALSLEY_ENABLED_LOCATIONS.lock().unwrap());
                continue; // skip
            }

            if is_reservable {
                info!(
                    "checking office {} as it appears reservable",
                    office_availability.office_name
                );
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
                                        _ => continue, // This case is unexpected.
                                    };

                                    if let Some(date) = NaiveDate::from_ymd_opt(year, month, day) {
                                        office_availability.available_dates.push(date);
                                    }
                                }
                            }
                        }
                    }

                    // --- Modification: Select the soonest date from the user-provided list ---
                    if !office_availability.available_dates.is_empty() {
                        // Convert provided date strings to NaiveDate objects.
                        let provided_dates: Vec<NaiveDate> = dates
                            .iter()
                            .filter_map(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
                            .collect();

                        // Filter the scraped available dates for those that also appear in the user's list.
                        let mut matching_dates: Vec<NaiveDate> = office_availability
                            .available_dates
                            .iter()
                            .filter(|date| provided_dates.contains(date))
                            .copied()
                            .collect();

                        if matching_dates.is_empty() {
                            if let Ok(back_button) = driver.find(By::Id("BackButton")).await {
                                let _ = back_button.click().await;
                                sleep(Duration::from_secs(1)).await;
                            }
                            FALSLEY_ENABLED_LOCATIONS
                                .lock()
                                .unwrap()
                                .push(office_availability.office_name);

                            break;
                        }

                        // Sort the matching dates so that the soonest is first.
                        matching_dates.sort();

                        if let Some(earliest_date) = matching_dates.first() {
                            // Get the day as text.
                            let day_text = earliest_date.day().to_string();

                            // Try to find and click the date on the page.
                            if driver.find(By::LinkText(&day_text)).await.is_ok() {
                                info!(
                                    "Selected date {} for office {}",
                                    earliest_date, office_availability.office_name
                                );
                            }
                        }
                    }
                    // ------------------------------------------------------------------------

                    if let Ok(next_button) = driver.find(By::ClassName("next-button")).await {
                        let _ = next_button.click().await;
                        sleep(Duration::from_secs(1)).await;
                    }

                    // Check for various "no availability" messages and back out if needed.
                    if driver
                        .find(By::Tag("body"))
                        .await
                        .expect("failed to read pages text")
                        .text()
                        .await
                        .unwrap()
                        .contains("This office does not currently have any appointments available in the next 90 days. Please try scheduling an appointment at another office or try again tomorrow when a new day's appointments will be available.")
                    {
                        if let Ok(back_button) = driver.find(By::Id("BackButton")).await {
                            let _ = back_button.click().await;
                            sleep(Duration::from_secs(1)).await;
                        }
                        FALSLEY_ENABLED_LOCATIONS.lock().unwrap().push(office_availability.office_name);
                        break;
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
                        if let Ok(back_button) = driver.find(By::Id("BackButton")).await {
                            let _ = back_button.click().await;
                            sleep(Duration::from_secs(1)).await;
                        }
                        FALSLEY_ENABLED_LOCATIONS
                            .lock()
                            .unwrap()
                            .push(office_availability.office_name);
                        break;
                    }

                    if driver
                        .find(By::Tag("body"))
                        .await
                        .expect("failed to read pages text")
                        .text()
                        .await
                        .unwrap()
                        .contains("We were unable")
                    {
                        if let Ok(back_button) = driver.find(By::Id("BackButton")).await {
                            let _ = back_button.click().await;
                            sleep(Duration::from_secs(1)).await;
                        }
                        FALSLEY_ENABLED_LOCATIONS
                            .lock()
                            .unwrap()
                            .push(office_availability.office_name);
                        break;
                    }

                    sleep(Duration::from_secs(3)).await;

                    let name_clone = self.name.clone();
                    let names: Vec<&str> = name_clone.split('_').collect();
                    let fname = names[0];
                    let lname = if names.len() > 1 { names[1] } else { "" };

                    driver.find(By::Id(FNAME_INPUT_ID)).await?.click().await?;
                    sleep(Duration::from_millis(150)).await;
                    driver
                        .find(By::Id(FNAME_INPUT_ID))
                        .await?
                        .send_keys(fname)
                        .await?;

                    sleep(Duration::from_millis(150)).await;
                    driver.find(By::Id(LNAME_INPUT_ID)).await?.click().await?;
                    sleep(Duration::from_millis(150)).await;
                    driver
                        .find(By::Id(LNAME_INPUT_ID))
                        .await?
                        .send_keys(lname)
                        .await?;

                    sleep(Duration::from_millis(150)).await;
                    driver
                        .find(By::Id(PHONE_NUM_INPUT_ID))
                        .await?
                        .click()
                        .await?;
                    sleep(Duration::from_millis(150)).await;
                    driver
                        .find(By::Id(PHONE_NUM_INPUT_ID))
                        .await?
                        .send_keys(self.phone_number.as_str())
                        .await?;

                    sleep(Duration::from_millis(150)).await;
                    driver.find(By::Id(EMAIL_INPUT_ID)).await?.click().await?;
                    sleep(Duration::from_millis(150)).await;

                    let last_date = Self::latest_date(dates.clone()).await.unwrap();
                    let proxy_email = Self::register_proxy_email(
                        &self.email,
                        &last_date,
                        "http://localhost:8000",
                    )
                    .await
                    .unwrap();

                    driver
                        .find(By::Id(EMAIL_INPUT_ID))
                        .await?
                        .send_keys(&proxy_email)
                        .await?;

                    sleep(Duration::from_millis(150)).await;
                    driver
                        .find(By::Id(CONFIRM_EMAIL_INPUT_ID))
                        .await?
                        .click()
                        .await?;
                    sleep(Duration::from_millis(150)).await;
                    driver
                        .find(By::Id(CONFIRM_EMAIL_INPUT_ID))
                        .await?
                        .send_keys(proxy_email)
                        .await?;

                    info!("solving captcha");
                    dotenv().ok();
                    let key = std::env::var("TWOCAPTCHA_KEY").expect("no 2captcha key set");
                    let solver = CaptchaSolver::new(key);

                    let args = RecaptchaV2::builder()
                        .website_url("https://skiptheline.ncdot.gov/")
                        .website_key("6LegSQ0dAAAAALO2_3-EDnTRDc7AQLz6Jo1BFyct")
                        .build()
                        .expect("failed to solve captcha");

                    let solution = solver
                        .solve(args)
                        .await
                        .expect("failed to solve captcha...")
                        .unwrap()
                        .solution;

                    let token = solution.g_recaptcha_response;

                    info!("got solution sucessfully!");

                    info!("{}", token);

                    let js = r#"
                        document.getElementById('g-recaptcha-response').innerHTML = arguments[0];
                        document.getElementById('g-recaptcha-response').style.display = 'block';
                    "#;

                    info!("executing js for captcha");

                    let args: Vec<Value> = vec![Value::String(token.to_string())];
                    driver.execute(js, Arc::from(args)).await?;

                    let js_callback = r#"
                        CaptchaCallBack(arguments[0]);
                    "#;

                    driver
                        .execute(
                            js_callback,
                            Arc::from(vec![Value::String(token.to_string())]),
                        )
                        .await?;

                    sleep(Duration::from_secs(1)).await;

                    if let Ok(next_button) = driver.find(By::ClassName("next-button")).await {
                        let _ = next_button.click().await;
                        sleep(Duration::from_secs(1)).await;
                    }

                    if let Ok(next_button) = driver.find(By::ClassName("next-button")).await {
                        let _ = next_button.click().await;
                        sleep(Duration::from_secs(1)).await;
                    }

                    break;
                }
            } else {
                if FALSLEY_ENABLED_LOCATIONS
                    .lock()
                    .unwrap()
                    .contains(&office_availability.office_name)
                {
                    info!("clearing locations...");
                    FALSLEY_ENABLED_LOCATIONS
                        .lock()
                        .unwrap()
                        .retain(|x| x != &office_availability.office_name);
                }
            }

            results.push(office_availability);
        }

        Ok(results)
    }

    async fn latest_date(dates: Vec<String>) -> Option<String> {
        dates
            .into_iter()
            .filter_map(|s| {
                NaiveDate::parse_from_str(&s, "%Y-%m-%d")
                    .ok()
                    .map(|d| (d, s))
            })
            .max_by_key(|(d, _)| *d)
            .map(|(_, original)| original)
    }

    async fn register_proxy_email(
        real_email: &str,
        expire_date: &str,
        api_url: &str,
    ) -> Result<String, Box<dyn Error>> {
        let client = Client::new();
        let body = RegisterRequest {
            real_email,
            expire_date,
        };

        let res = client
            .post(format!("{}/register", api_url))
            .json(&body)
            .send()
            .await?;

        if res.status().is_success() {
            let parsed: crate::models::email::RegisterResponse = res.json().await?;
            Ok(parsed.proxy_email)
        } else {
            let error_text = res.text().await?;
            Err(format!("API error: {}", error_text).into())
        }
    }

    pub async fn start_appointment_stream(
        self: Arc<Self>,
        refresh_interval_secs: u64,
        service_type: DMVService,
        dates: Vec<String>,
    ) -> mpsc::Receiver<Vec<OfficeAvailability>> {
        let (tx, rx) = mpsc::channel(117); // Buffer size of 117 for 117 DMVs in NC
        info!("scraping NC DMV data with date checking");

        // Clone shared state to move into the task
        let scraper = Arc::clone(&self);
        let zip_code = scraper.zipcode.clone();

        tokio::spawn(async move {
            if let Err(e) = scraper
                .stream_available_appointments(
                    zip_code,
                    refresh_interval_secs,
                    tx,
                    service_type.selector().to_string(),
                    dates,
                )
                .await
            {
                error!("Error starting appointment stream: {:?}", e);
            }
        });

        rx
    }
}
