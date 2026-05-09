use super::*;

pub(crate) const PROTOCOL_VERSION: &str = "1.5";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProtocolMethod {
    PackagesListInstalled,
    PackagesListAvailable,
    PackagesListPulse,
    PackagesInfo,
    PackagesSearch,
    PackagesListOutdated,
    PackagesHomebrewMigrationRecommendation,
    PackagesIsotopeMigrationPlan,
    PackagesMigrateIsotope,
    PackagesMakeDefault,
    SystemInfo,
}

impl ProtocolMethod {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "packages.listInstalled" => Some(Self::PackagesListInstalled),
            "packages.listAvailable" => Some(Self::PackagesListAvailable),
            "packages.listPulse" => Some(Self::PackagesListPulse),
            "packages.info" => Some(Self::PackagesInfo),
            "packages.search" => Some(Self::PackagesSearch),
            "packages.listOutdated" => Some(Self::PackagesListOutdated),
            "packages.homebrewMigrationRecommendation" => {
                Some(Self::PackagesHomebrewMigrationRecommendation)
            }
            "packages.isotopeMigrationPlan" => Some(Self::PackagesIsotopeMigrationPlan),
            "packages.migrateIsotope" => Some(Self::PackagesMigrateIsotope),
            "packages.makeDefault" => Some(Self::PackagesMakeDefault),
            "system.info" => Some(Self::SystemInfo),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProtocolRequest {
    pub(crate) id: u64,
    pub(crate) method: String,
    #[serde(default)]
    pub(crate) params: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub(crate) struct ProtocolSuccessResponse<T> {
    pub(crate) id: u64,
    pub(crate) result: T,
}

#[derive(Debug, Serialize)]
pub(crate) struct ProtocolErrorResponse {
    pub(crate) id: u64,
    pub(crate) error: ProtocolError,
}

#[derive(Debug, Serialize)]
pub(crate) struct ProtocolError {
    pub(crate) code: i32,
    pub(crate) message: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct ListInstalledResponse {
    pub(crate) packages: Vec<InstalledPackageSummary>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct InstalledPackageSummary {
    pub(crate) name: String,
    pub(crate) source: PackageReceiptSource,
    pub(crate) version: String,
    pub(crate) description: Option<String>,
    #[serde(rename = "installedVersions", skip_serializing_if = "Vec::is_empty")]
    pub(crate) installed_versions: Vec<String>,
    #[serde(rename = "installPackageNames", skip_serializing_if = "Vec::is_empty")]
    pub(crate) install_package_names: Vec<String>,
    #[serde(rename = "securityState")]
    pub(crate) security_state: Option<PackageSecurityState>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct SearchPackagesResponse {
    pub(crate) packages: Vec<SearchPackageSummary>,
    #[serde(rename = "totalCount")]
    pub(crate) total_count: usize,
    #[serde(rename = "nextOffset")]
    pub(crate) next_offset: Option<usize>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct SearchPackageSummary {
    pub(crate) name: String,
    pub(crate) source: PackageReceiptSource,
    pub(crate) version: Option<String>,
    pub(crate) description: Option<String>,
    #[serde(rename = "securityState")]
    pub(crate) security_state: Option<PackageSecurityState>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct ListOutdatedResponse {
    pub(crate) packages: Vec<OutdatedPackageSummary>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct HomebrewMigrationRecommendationResponse {
    pub(crate) packages: Vec<HomebrewMigrationPackageSummary>,
    pub(crate) hazards: Vec<HomebrewMigrationHazardSummary>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct HomebrewMigrationPackageSummary {
    pub(crate) name: String,
    pub(crate) version: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) tap: Option<String>,
    #[serde(rename = "isMigratable")]
    pub(crate) is_migratable: bool,
    #[serde(rename = "securityState")]
    pub(crate) security_state: Option<PackageSecurityState>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct HomebrewMigrationHazardSummary {
    #[serde(rename = "packageName")]
    pub(crate) package_name: String,
    #[serde(rename = "isotopeName")]
    pub(crate) isotope_name: String,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct OutdatedPackageSummary {
    #[serde(rename = "currentVersion")]
    pub(crate) current_version: String,
    #[serde(rename = "latestVersion")]
    pub(crate) latest_version: String,
    pub(crate) name: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct SystemInfoResponse {
    #[serde(rename = "protocolVersion")]
    pub(crate) protocol_version: &'static str,
    pub(crate) version: &'static str,
    #[serde(rename = "buildId")]
    pub(crate) build_id: &'static str,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct IsotopeMigrationPlanResponse {
    #[serde(rename = "isotopeName")]
    pub(crate) isotope_name: String,
    #[serde(rename = "replacesPackage")]
    pub(crate) replaces_package: Option<String>,
    #[serde(rename = "modifiesPackage")]
    pub(crate) modifies_package: Option<String>,
    #[serde(rename = "isRadioisotope")]
    pub(crate) is_radioisotope: bool,
    #[serde(rename = "hasMigration")]
    pub(crate) has_migration: bool,
}

pub(crate) fn success_response<T>(id: u64, result: T) -> ProtocolSuccessResponse<T> {
    ProtocolSuccessResponse { id, result }
}

pub(crate) fn error_response(
    id: u64,
    code: i32,
    message: impl Into<String>,
) -> ProtocolErrorResponse {
    ProtocolErrorResponse {
        id,
        error: ProtocolError {
            code,
            message: message.into(),
        },
    }
}
