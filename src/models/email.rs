use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct RegisterRequest<'a> {
    pub real_email: &'a str,
    pub expire_date: &'a str, // ISO 8601
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RegisterResponse {
    pub proxy_email: String,
}
