/// Represents different types of driver's license services
#[derive(Debug, Clone, PartialEq)]
pub enum DMVService {
    /// First time application for a driver's license
    FirstTime {
        title: &'static str,
        selector: &'static str,
    },
    /// Duplicate of an existing license
    Duplicate {
        title: &'static str,
        selector: &'static str,
    },
    /// Renewal of an existing license
    Renewal {
        title: &'static str,
        selector: &'static str,
    },
    /// Fee-related services
    Fees {
        title: &'static str,
        selector: &'static str,
    },
    /// ID card services
    IdCard {
        title: &'static str,
        selector: &'static str,
    },
    /// Knowledge and computer test services
    KnowledgeTest {
        title: &'static str,
        selector: &'static str,
    },
    /// Legal presence verification for non-citizens
    LegalPresence {
        title: &'static str,
        selector: &'static str,
    },
    /// Motorcycle skills test scheduling
    MotorcycleTest {
        title: &'static str,
        selector: &'static str,
    },
    /// Non-CDL road test scheduling
    NonCdlRoadTest {
        title: &'static str,
        selector: &'static str,
    },
    /// Permit services
    Permits {
        title: &'static str,
        selector: &'static str,
    },
    /// Teen driver level 1 services
    TeenDriverLevel1 {
        title: &'static str,
        selector: &'static str,
    },
    /// Teen driver level 2 services
    TeenDriverLevel2 {
        title: &'static str,
        selector: &'static str,
    },
    /// Teen driver level 3 services
    TeenDriverLevel3 {
        title: &'static str,
        selector: &'static str,
    },
}

/// Implementation for DMVService
impl DMVService {
    /// Gets the title of the service
    pub fn _title(&self) -> &'static str {
        match self {
            DMVService::FirstTime { title, .. } => title,
            DMVService::Duplicate { title, .. } => title,
            DMVService::Renewal { title, .. } => title,
            DMVService::Fees { title, .. } => title,
            DMVService::IdCard { title, .. } => title,
            DMVService::KnowledgeTest { title, .. } => title,
            DMVService::LegalPresence { title, .. } => title,
            DMVService::MotorcycleTest { title, .. } => title,
            DMVService::NonCdlRoadTest { title, .. } => title,
            DMVService::Permits { title, .. } => title,
            DMVService::TeenDriverLevel1 { title, .. } => title,
            DMVService::TeenDriverLevel2 { title, .. } => title,
            DMVService::TeenDriverLevel3 { title, .. } => title,
        }
    }

    /// Gets the selector string of the service
    pub fn selector(&self) -> &'static str {
        match self {
            DMVService::FirstTime { selector, .. } => selector,
            DMVService::Duplicate { selector, .. } => selector,
            DMVService::Renewal { selector, .. } => selector,
            DMVService::Fees { selector, .. } => selector,
            DMVService::IdCard { selector, .. } => selector,
            DMVService::KnowledgeTest { selector, .. } => selector,
            DMVService::LegalPresence { selector, .. } => selector,
            DMVService::MotorcycleTest { selector, .. } => selector,
            DMVService::NonCdlRoadTest { selector, .. } => selector,
            DMVService::Permits { selector, .. } => selector,
            DMVService::TeenDriverLevel1 { selector, .. } => selector,
            DMVService::TeenDriverLevel2 { selector, .. } => selector,
            DMVService::TeenDriverLevel3 { selector, .. } => selector,
        }
    }
}
