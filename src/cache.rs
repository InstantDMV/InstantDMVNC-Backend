use moka::future::Cache;
use once_cell::sync::Lazy;
use std::sync::Arc;

use crate::models::offices::OfficeAvailability;

pub static OFFICE_CACHE: Lazy<Arc<Cache<String, OfficeAvailability>>> = Lazy::new(|| {
    Arc::new(Cache::new(117)) // 117 dmvs in nc
});
