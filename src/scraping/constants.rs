// thirtyfour (selenium) inputs
pub const BASE_URL: &str =
    "https://skiptheline.ncdot.gov/Webapp/Appointment/Index/a7ade79b-996d-4971-8766-97feb75254de";

pub const WINDOW_WIDTH: u32 = 1280;
pub const WINDOW_HEIGHT: u32 = 775;

// HTML element selectors used in automation
pub const BUTTON_MAKE_APPT_ID: &str = "cmdMakeAppt";
// pub const SEARCH_INPUT_ID: &str = "search-input";
// pub const INPUT_RESULTS_SELECTOR: &str = ".input-results";

// HTML class selectors for scraping
pub const DMV_ITEM_CLASS: &str = "QflowObjectItem";
pub const DMV_CHILD_CLASS: &str = "form-control-child";
pub const ACTIVE_UNIT_CLASS: &str = "Active-Unit";
pub const AVAILABLE_DATE_CLASS: &str = "ui-state-default ui-state-active";

//IDS of info input fields
pub const FNAME_INPUT_ID: &str = "StepControls_0__Model_Value_Properties_0__Value";
pub const LNAME_INPUT_ID: &str = "StepControls_0__Model_Value_Properties_1__Value";
pub const PHONE_NUM_INPUT_ID: &str = "StepControls_0__Model_Value_Properties_2__Value";
pub const EMAIL_INPUT_ID: &str = "StepControls_0__Model_Value_Properties_3__Value";
pub const CONFIRM_EMAIL_INPUT_ID: &str = "StepControls_0__Model_Value_Properties_4__Value";
