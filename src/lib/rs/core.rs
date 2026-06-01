use super::*;
use std::collections::BTreeMap;

pub(crate) const PROTOCOL_VERSION: &str = "1.13";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProtocolMethod {
    PackagesListInstalled,
    PackagesListAvailable,
    PackagesListPulse,
    PackagesListGeiger,
    PackagesInfo,
    PackagesSearch,
    PackagesListOutdated,
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
            "packages.listGeiger" => Some(Self::PackagesListGeiger),
            "packages.info" => Some(Self::PackagesInfo),
            "packages.search" => Some(Self::PackagesSearch),
            "packages.listOutdated" => Some(Self::PackagesListOutdated),
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) homepage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) repository: Option<String>,
    #[serde(rename = "upstreamDocs", skip_serializing_if = "Option::is_none")]
    pub(crate) upstream_docs: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) docs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) category: Option<String>,
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
    #[serde(rename = "categoryCounts", skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) category_counts: BTreeMap<String, usize>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct SearchPackageSummary {
    pub(crate) name: String,
    pub(crate) source: PackageReceiptSource,
    pub(crate) version: Option<String>,
    pub(crate) description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) homepage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) repository: Option<String>,
    #[serde(rename = "upstreamDocs", skip_serializing_if = "Option::is_none")]
    pub(crate) upstream_docs: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) docs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) category: Option<String>,
    #[serde(rename = "lastUpdatedAt", skip_serializing_if = "Option::is_none")]
    pub(crate) last_updated_at: Option<String>,
    #[serde(rename = "pulseKind", skip_serializing_if = "Option::is_none")]
    pub(crate) pulse_kind: Option<String>,
    #[serde(rename = "securityState")]
    pub(crate) security_state: Option<PackageSecurityState>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct ListOutdatedResponse {
    pub(crate) packages: Vec<OutdatedPackageSummary>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_recognizes_supported_methods() {
        let cases = [
            (
                "packages.listInstalled",
                ProtocolMethod::PackagesListInstalled,
            ),
            (
                "packages.listAvailable",
                ProtocolMethod::PackagesListAvailable,
            ),
            ("packages.listPulse", ProtocolMethod::PackagesListPulse),
            ("packages.listGeiger", ProtocolMethod::PackagesListGeiger),
            ("packages.info", ProtocolMethod::PackagesInfo),
            ("packages.search", ProtocolMethod::PackagesSearch),
            (
                "packages.listOutdated",
                ProtocolMethod::PackagesListOutdated,
            ),
            (
                "packages.isotopeMigrationPlan",
                ProtocolMethod::PackagesIsotopeMigrationPlan,
            ),
            (
                "packages.migrateIsotope",
                ProtocolMethod::PackagesMigrateIsotope,
            ),
            ("packages.makeDefault", ProtocolMethod::PackagesMakeDefault),
            ("system.info", ProtocolMethod::SystemInfo),
        ];

        for (value, expected) in cases {
            assert_eq!(ProtocolMethod::parse(value), Some(expected));
        }
    }

    #[test]
    fn parse_rejects_unknown_methods() {
        assert_eq!(ProtocolMethod::parse("packages.unknown"), None);
    }

    #[test]
    fn protocol_request_defaults_missing_params_to_null() {
        let request: ProtocolRequest =
            serde_json::from_value(json!({ "id": 42, "method": "packages.info" })).unwrap();

        assert_eq!(request.id, 42);
        assert_eq!(request.method, "packages.info");
        assert_eq!(request.params, serde_json::Value::Null);
    }

    #[test]
    fn installed_package_summary_serializes_public_field_names() {
        let summary = InstalledPackageSummary {
            name: "openssl".to_string(),
            source: PackageReceiptSource::Formula {
                root_formula: "openssl@3".to_string(),
            },
            version: "3.0.0".to_string(),
            description: Some("TLS toolkit".to_string()),
            homepage: Some("https://openssl-library.org".to_string()),
            repository: Some("https://github.com/openssl/openssl".to_string()),
            upstream_docs: Some("https://docs.openssl.org/".to_string()),
            docs: vec!["https://docs.openssl.org/".to_string()],
            category: Some("security".to_string()),
            installed_versions: vec!["3.0.0".to_string()],
            install_package_names: vec!["brew:openssl@3".to_string()],
            security_state: Some(PackageSecurityState {
                isotope_name: "openssl".to_string(),
                install_is_insecure: false,
                remediation_available: true,
                reasons: vec!["covered".to_string()],
                error: None,
            }),
        };

        let value = serde_json::to_value(summary).unwrap();
        assert_eq!(value["homepage"], "https://openssl-library.org");
        assert_eq!(value["repository"], "https://github.com/openssl/openssl");
        assert_eq!(value["upstreamDocs"], "https://docs.openssl.org/");
        assert_eq!(value["docs"], json!(["https://docs.openssl.org/"]));
        assert_eq!(value["category"], "security");
        assert_eq!(value["installedVersions"], json!(["3.0.0"]));
        assert_eq!(value["installPackageNames"], json!(["brew:openssl@3"]));
        assert_eq!(
            value["source"],
            json!({ "kind": "formula", "root_formula": "openssl@3" })
        );
        assert_eq!(value["securityState"]["isotopeName"], "openssl");
    }

    #[test]
    fn response_types_use_expected_json_field_names() {
        let search = SearchPackagesResponse {
            packages: vec![SearchPackageSummary {
                name: "pkg".to_string(),
                source: PackageReceiptSource::Npm {
                    package_name: "pkg".to_string(),
                },
                version: Some("1.2.3".to_string()),
                description: None,
                homepage: Some("https://example.test/pkg".to_string()),
                repository: Some("https://github.com/example/pkg".to_string()),
                upstream_docs: Some("https://docs.example.test/pkg".to_string()),
                docs: vec!["https://docs.example.test/pkg".to_string()],
                category: Some("developer-tools".to_string()),
                last_updated_at: Some("2026-05-27T12:00:00Z".to_string()),
                pulse_kind: Some("release".to_string()),
                security_state: None,
            }],
            total_count: 1,
            next_offset: Some(25),
            category_counts: BTreeMap::from([("developer-tools".to_string(), 1)]),
        };
        let search_json = serde_json::to_value(search).unwrap();
        assert_eq!(search_json["totalCount"], 1);
        assert_eq!(search_json["nextOffset"], 25);
        assert_eq!(
            search_json["packages"][0]["lastUpdatedAt"],
            "2026-05-27T12:00:00Z"
        );
        assert_eq!(
            search_json["packages"][0]["homepage"],
            "https://example.test/pkg"
        );
        assert_eq!(
            search_json["packages"][0]["repository"],
            "https://github.com/example/pkg"
        );
        assert_eq!(
            search_json["packages"][0]["upstreamDocs"],
            "https://docs.example.test/pkg"
        );
        assert_eq!(search_json["packages"][0]["category"], "developer-tools");
        assert_eq!(search_json["categoryCounts"]["developer-tools"], 1);
        assert_eq!(search_json["packages"][0]["pulseKind"], "release");

        let plan = IsotopeMigrationPlanResponse {
            isotope_name: "gh".to_string(),
            replaces_package: Some("brew:gh".to_string()),
            modifies_package: None,
            is_radioisotope: true,
            has_migration: false,
        };
        let plan_json = serde_json::to_value(plan).unwrap();
        assert_eq!(plan_json["isotopeName"], "gh");
        assert_eq!(plan_json["replacesPackage"], "brew:gh");
        assert_eq!(plan_json["isRadioisotope"], true);
        assert_eq!(plan_json["hasMigration"], false);
    }

    #[test]
    fn response_helpers_wrap_payloads() {
        let success = success_response(7, json!({ "ok": true }));
        let success_json = serde_json::to_value(success).unwrap();
        assert_eq!(success_json, json!({ "id": 7, "result": { "ok": true } }));

        let error = error_response(9, -32000, "boom");
        let error_json = serde_json::to_value(error).unwrap();
        assert_eq!(
            error_json,
            json!({ "id": 9, "error": { "code": -32000, "message": "boom" } })
        );
    }
}
