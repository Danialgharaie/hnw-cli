use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Site {
    pub slug: String,
    pub site_url: String,
    pub updated_at: Option<String>,
    pub expires_at: Option<String>,
    pub status: Option<String>,
    pub current_version_id: Option<String>,
    pub pending_version_id: Option<String>,
    pub display_name: Option<String>,
    pub display_description: Option<String>,
    #[serde(default)]
    pub manifest: Vec<ManifestEntry>,
}

impl Site {
    pub fn label(&self) -> &str {
        self.display_name.as_deref().unwrap_or(&self.slug)
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestEntry {
    pub path: String,
    pub size: u64,
    pub content_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SitesResponse {
    pub publishes: Vec<Site>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub slug: String,
    pub site_url: String,
    pub primary_url: Option<String>,
    pub display_name: Option<String>,
    pub updated_at: Option<String>,
    #[serde(default)]
    pub matched_paths: Vec<String>,
    pub snippet: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Drive {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub description: Option<String>,
    pub status: String,
    pub head_version_id: Option<String>,
    pub dashboard_url: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DrivesResponse {
    pub drives: Vec<Drive>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveFile {
    pub path: String,
    pub size: Option<u64>,
    pub etag: Option<String>,
    pub content_type: Option<String>,
    pub updated_at: Option<String>,
    pub last_modified_by: Option<String>,
    pub last_operation: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DriveFilesResponse {
    #[serde(default)]
    pub files: Vec<DriveFile>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub username: String,
    pub enabled: bool,
    pub add_new_sites_to_profile: bool,
    pub url: String,
    pub feed_url: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Analytics {
    pub range: String,
    pub last_event_at: Option<String>,
    pub totals: AnalyticsTotals,
    #[serde(default)]
    pub top_paths: Vec<MetricRow>,
    #[serde(default)]
    pub top_referrers: Vec<MetricRow>,
    #[serde(default)]
    pub top_countries: Vec<MetricRow>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsTotals {
    pub all_time_views: Option<u64>,
    pub range_views: Option<u64>,
    pub range_visitors: Option<u64>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct MetricRow {
    pub path: Option<String>,
    pub referrer: Option<String>,
    pub country: Option<String>,
    pub views: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataPatch<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_description: Option<&'a str>,
}
