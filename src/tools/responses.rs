use serde::{Deserialize, Serialize};

use crate::models::{UpdateType, VersionStability};

/// Response for get_latest_version tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestVersionResponse {
    pub dependency: String,
    pub latest: LatestVersions,
    pub total_versions: usize,
    pub stable_versions: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_updated: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestVersions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stable: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub beta: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alpha: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub milestone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any: Option<String>,
}

/// Response for check_version_exists tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionExistsResponse {
    pub dependency: String,
    pub version: String,
    pub exists: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stability: Option<VersionStability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_stable: Option<String>,
}

/// Response for compare_versions tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionComparisonResponse {
    pub dependency: String,
    pub current_version: String,
    pub current_stability: VersionStability,
    pub latest_version: String,
    pub latest_stability: VersionStability,
    pub is_outdated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_type: Option<UpdateType>,
    pub versions_behind: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommendation: Option<String>,
}

/// Response for check_multiple_dependencies tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkCheckResponse {
    pub total_checked: usize,
    pub outdated_count: usize,
    pub up_to_date_count: usize,
    pub error_count: usize,
    pub dependencies: Vec<DependencyCheckResult>,
    pub summary: BulkCheckSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyCheckResult {
    pub dependency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<String>,
    pub is_outdated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_type: Option<UpdateType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkCheckSummary {
    pub major_updates: usize,
    pub minor_updates: usize,
    pub patch_updates: usize,
}

/// Response for analyze_dependency_age tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyAgeResponse {
    pub dependency: String,
    pub version: String,
    pub age_classification: AgeClassification,
    pub versions_since: usize,
    pub stable_versions_since: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_stable: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommendation: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgeClassification {
    /// Using the latest version
    Current,
    /// 1-2 versions behind
    Fresh,
    /// 3-5 versions behind
    Aging,
    /// More than 5 versions behind
    Stale,
    /// Using a major version behind
    Outdated,
}

impl std::fmt::Display for AgeClassification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgeClassification::Current => write!(f, "current"),
            AgeClassification::Fresh => write!(f, "fresh"),
            AgeClassification::Aging => write!(f, "aging"),
            AgeClassification::Stale => write!(f, "stale"),
            AgeClassification::Outdated => write!(f, "outdated"),
        }
    }
}

/// Response for analyze_project_health tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectHealthResponse {
    pub total_dependencies: usize,
    pub health_score: f32,
    pub health_grade: HealthGrade,
    pub summary: HealthSummary,
    pub dependencies: Vec<DependencyHealthResult>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSummary {
    pub current: usize,
    pub fresh: usize,
    pub aging: usize,
    pub stale: usize,
    pub outdated: usize,
    pub errors: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyHealthResult {
    pub dependency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age_classification: Option<AgeClassification>,
    pub health_score: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_type: Option<UpdateType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthGrade {
    A,
    B,
    C,
    D,
    F,
}

impl HealthGrade {
    pub fn from_score(score: f32) -> Self {
        match score {
            s if s >= 90.0 => HealthGrade::A,
            s if s >= 80.0 => HealthGrade::B,
            s if s >= 70.0 => HealthGrade::C,
            s if s >= 60.0 => HealthGrade::D,
            _ => HealthGrade::F,
        }
    }
}

impl std::fmt::Display for HealthGrade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthGrade::A => write!(f, "A"),
            HealthGrade::B => write!(f, "B"),
            HealthGrade::C => write!(f, "C"),
            HealthGrade::D => write!(f, "D"),
            HealthGrade::F => write!(f, "F"),
        }
    }
}
