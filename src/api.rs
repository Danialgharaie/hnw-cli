use std::{env, fs, path::PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use reqwest::{Client, Method, StatusCode};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};

use crate::model::{
    Analytics, DriveFile, DriveFilesResponse, DrivesResponse, MetadataPatch, Profile,
    SearchResponse, Site, SitesResponse,
};

#[derive(Clone)]
pub struct HereNowClient {
    http: Client,
    base_url: String,
    api_key: String,
}

impl HereNowClient {
    pub fn from_credentials(base_url: String, path: Option<PathBuf>) -> Result<Self> {
        let api_key = if let Ok(key) = env::var("HERENOW_API_KEY") {
            key
        } else {
            let path = path.or_else(default_credentials_path).ok_or_else(|| {
                anyhow!("no home directory; pass --credentials or HERENOW_API_KEY")
            })?;
            fs::read_to_string(&path)
                .with_context(|| format!("could not read {}", path.display()))?
        };
        let api_key = api_key.trim().to_owned();
        if api_key.is_empty() {
            bail!("here.now API key is empty")
        }
        Ok(Self {
            http: Client::builder().user_agent("hnw/0.1.0").build()?,
            base_url: base_url.trim_end_matches('/').to_owned(),
            api_key,
        })
    }

    async fn request<T: DeserializeOwned>(&self, method: Method, path: &str) -> Result<T> {
        let response = self
            .http
            .request(method, format!("{}{}", self.base_url, path))
            .bearer_auth(&self.api_key)
            .header("X-HereNow-Client", "hnw/tui")
            .send()
            .await?;
        parse_response(response).await
    }

    async fn request_json<B: Serialize, T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let response = self
            .http
            .request(method, format!("{}{}", self.base_url, path))
            .bearer_auth(&self.api_key)
            .header("X-HereNow-Client", "hnw/tui")
            .json(body)
            .send()
            .await?;
        parse_response(response).await
    }

    pub async fn sites(&self) -> Result<Vec<Site>> {
        Ok(self
            .request::<SitesResponse>(Method::GET, "/api/v1/publishes")
            .await?
            .publishes)
    }

    pub async fn site(&self, slug: &str) -> Result<Site> {
        self.request(Method::GET, &format!("/api/v1/publish/{slug}"))
            .await
    }

    pub async fn search(&self, query: &str) -> Result<Vec<Site>> {
        let encoded = urlencoding::encode(query);
        let results = self
            .request::<SearchResponse>(
                Method::GET,
                &format!("/api/v1/publishes/search?q={encoded}&limit=100"),
            )
            .await?
            .results;
        Ok(results
            .into_iter()
            .map(|item| Site {
                slug: item.slug,
                site_url: item.primary_url.unwrap_or(item.site_url),
                updated_at: item.updated_at,
                display_name: item.display_name,
                display_description: item.snippet,
                manifest: item
                    .matched_paths
                    .into_iter()
                    .map(|path| crate::model::ManifestEntry {
                        path,
                        ..Default::default()
                    })
                    .collect(),
                ..Default::default()
            })
            .collect())
    }

    pub async fn patch_metadata(
        &self,
        slug: &str,
        name: &str,
        description: Option<&str>,
    ) -> Result<()> {
        let _: Value = self
            .request_json(
                Method::PATCH,
                &format!("/api/v1/publish/{slug}/metadata"),
                &MetadataPatch {
                    display_name: Some(name),
                    display_description: description,
                },
            )
            .await?;
        Ok(())
    }

    pub async fn duplicate(&self, slug: &str) -> Result<Site> {
        self.request_json(
            Method::POST,
            &format!("/api/v1/publish/{slug}/duplicate"),
            &json!({}),
        )
        .await
    }

    pub async fn delete_site(&self, slug: &str) -> Result<()> {
        let response = self
            .http
            .delete(format!("{}/api/v1/publish/{slug}", self.base_url))
            .bearer_auth(&self.api_key)
            .header("X-HereNow-Client", "hnw/tui")
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(response_error(response).await);
        }
        Ok(())
    }

    pub async fn analytics(&self, slug: &str, range: &str) -> Result<Analytics> {
        self.request(
            Method::GET,
            &format!("/api/v1/publishes/{slug}/analytics?range={range}"),
        )
        .await
    }

    pub async fn drives(&self) -> Result<DrivesResponse> {
        self.request(Method::GET, "/api/v1/drives").await
    }

    pub async fn drive_files(&self, drive_id: &str) -> Result<Vec<DriveFile>> {
        Ok(self
            .request::<DriveFilesResponse>(
                Method::GET,
                &format!("/api/v1/drives/{drive_id}/files?prefix="),
            )
            .await?
            .files)
    }

    pub async fn profile(&self) -> Result<Profile> {
        self.request(Method::GET, "/api/v1/profile").await
    }
}

fn default_credentials_path() -> Option<PathBuf> {
    dirs::home_dir().map(|path| path.join(".herenow/credentials"))
}

async fn parse_response<T: DeserializeOwned>(response: reqwest::Response) -> Result<T> {
    if response.status().is_success() {
        return response.json().await.context("invalid here.now response");
    }
    Err(response_error(response).await)
}

async fn response_error(response: reqwest::Response) -> anyhow::Error {
    let status = response.status();
    let fallback = status.canonical_reason().unwrap_or("request failed");
    let body = response.text().await.unwrap_or_default();
    let message = serde_json::from_str::<Value>(&body)
        .ok()
        .and_then(|json| {
            json.get("message")
                .or_else(|| json.get("error"))
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| fallback.to_owned());
    if status == StatusCode::UNAUTHORIZED {
        anyhow!("authentication failed; check ~/.herenow/credentials")
    } else {
        anyhow!("here.now returned {status}: {message}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credentials_are_trimmed() {
        let file = tempfile::NamedTempFile::new().unwrap();
        fs::write(file.path(), "hnk_example\n").unwrap();
        let client = HereNowClient::from_credentials(
            "https://here.now/".into(),
            Some(file.path().to_path_buf()),
        )
        .unwrap();
        assert_eq!(client.api_key, "hnk_example");
        assert_eq!(client.base_url, "https://here.now");
    }
}
