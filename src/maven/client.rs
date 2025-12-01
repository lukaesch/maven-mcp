use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use moka::future::Cache;
use reqwest::Client;
use tracing::{debug, instrument};

use crate::maven::metadata::MavenMetadata;
use crate::models::{MavenCoordinate, MavenVersion, VersionStability};

const MAVEN_CENTRAL_BASE: &str = "https://repo1.maven.org/maven2";
const CACHE_TTL_HOURS: u64 = 24;
const CACHE_MAX_ENTRIES: u64 = 1000;

/// Client for fetching data from Maven Central
#[derive(Clone)]
pub struct MavenClient {
    http: Client,
    cache: Cache<String, Arc<CachedMetadata>>,
}

/// Cached metadata with processed version information
#[derive(Debug, Clone)]
pub struct CachedMetadata {
    pub all_versions: Vec<String>,
    pub stable_versions: Vec<String>,
    pub latest_stable: Option<String>,
    pub latest_any: Option<String>,
    pub latest_rc: Option<String>,
    pub latest_beta: Option<String>,
    pub latest_alpha: Option<String>,
    pub latest_milestone: Option<String>,
    pub last_updated: Option<String>,
}

impl MavenClient {
    pub fn new() -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("maven-central-mcp/0.1.0")
            .build()
            .expect("Failed to create HTTP client");

        let cache = Cache::builder()
            .max_capacity(CACHE_MAX_ENTRIES)
            .time_to_live(Duration::from_secs(CACHE_TTL_HOURS * 3600))
            .build();

        MavenClient { http, cache }
    }

    /// Fetch and process metadata for a Maven coordinate
    #[instrument(skip(self), fields(coordinate = %coordinate))]
    pub async fn get_metadata(&self, coordinate: &MavenCoordinate) -> Result<Arc<CachedMetadata>> {
        let cache_key = coordinate.to_ga();

        // Check cache first
        if let Some(cached) = self.cache.get(&cache_key).await {
            debug!("Cache hit for {}", cache_key);
            return Ok(cached);
        }

        debug!("Cache miss for {}, fetching from Maven Central", cache_key);

        // Fetch from Maven Central
        let metadata = self.fetch_metadata(coordinate).await?;
        let processed = Arc::new(self.process_metadata(&metadata));

        // Store in cache
        self.cache.insert(cache_key, processed.clone()).await;

        Ok(processed)
    }

    /// Fetch raw metadata from Maven Central
    async fn fetch_metadata(&self, coordinate: &MavenCoordinate) -> Result<MavenMetadata> {
        let url = format!(
            "{}/{}/maven-metadata.xml",
            MAVEN_CENTRAL_BASE,
            coordinate.metadata_path()
        );

        debug!("Fetching metadata from {}", url);

        let response = self
            .http
            .get(&url)
            .send()
            .await
            .context("Failed to fetch maven-metadata.xml")?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Failed to fetch metadata for {}: HTTP {}",
                coordinate,
                response.status()
            );
        }

        let xml = response
            .text()
            .await
            .context("Failed to read response body")?;

        MavenMetadata::parse(&xml).context("Failed to parse maven-metadata.xml")
    }

    /// Process raw metadata into categorized version information
    fn process_metadata(&self, metadata: &MavenMetadata) -> CachedMetadata {
        let all_versions = metadata.get_versions();

        // Sort versions by Maven version ordering (newest first)
        let mut sorted_versions: Vec<(String, MavenVersion)> = all_versions
            .iter()
            .map(|v| (v.clone(), MavenVersion::parse(v)))
            .collect();
        sorted_versions.sort_by(|a, b| b.1.cmp(&a.1));

        // Categorize versions
        let mut stable_versions = Vec::new();
        let mut latest_stable = None;
        let mut latest_any = None;
        let mut latest_rc = None;
        let mut latest_beta = None;
        let mut latest_alpha = None;
        let mut latest_milestone = None;

        for (version_str, parsed) in &sorted_versions {
            if latest_any.is_none() {
                latest_any = Some(version_str.clone());
            }

            match parsed.stability {
                VersionStability::Stable => {
                    stable_versions.push(version_str.clone());
                    if latest_stable.is_none() {
                        latest_stable = Some(version_str.clone());
                    }
                }
                VersionStability::RC => {
                    if latest_rc.is_none() {
                        latest_rc = Some(version_str.clone());
                    }
                }
                VersionStability::Beta => {
                    if latest_beta.is_none() {
                        latest_beta = Some(version_str.clone());
                    }
                }
                VersionStability::Alpha => {
                    if latest_alpha.is_none() {
                        latest_alpha = Some(version_str.clone());
                    }
                }
                VersionStability::Milestone => {
                    if latest_milestone.is_none() {
                        latest_milestone = Some(version_str.clone());
                    }
                }
                VersionStability::Snapshot => {
                    // Snapshots are typically not in Central, but handle them anyway
                }
            }
        }

        CachedMetadata {
            all_versions: sorted_versions.into_iter().map(|(v, _)| v).collect(),
            stable_versions,
            latest_stable,
            latest_any,
            latest_rc,
            latest_beta,
            latest_alpha,
            latest_milestone,
            last_updated: metadata.get_last_updated().map(String::from),
        }
    }

    /// Check if a specific version exists
    pub async fn version_exists(
        &self,
        coordinate: &MavenCoordinate,
        version: &str,
    ) -> Result<bool> {
        let metadata = self.get_metadata(coordinate).await?;
        Ok(metadata.all_versions.iter().any(|v| v == version))
    }

    /// Get versions filtered by stability
    pub async fn get_versions_by_stability(
        &self,
        coordinate: &MavenCoordinate,
        stability: VersionStability,
    ) -> Result<Vec<String>> {
        let metadata = self.get_metadata(coordinate).await?;

        let versions: Vec<String> = metadata
            .all_versions
            .iter()
            .filter(|v| VersionStability::classify(v) == stability)
            .cloned()
            .collect();

        Ok(versions)
    }
}

impl Default for MavenClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_process_metadata() {
        let client = MavenClient::new();

        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<metadata>
  <groupId>org.example</groupId>
  <artifactId>test</artifactId>
  <versioning>
    <latest>2.0.0-RC1</latest>
    <release>1.5.0</release>
    <versions>
      <version>1.0.0</version>
      <version>1.0.1</version>
      <version>1.5.0</version>
      <version>2.0.0-alpha</version>
      <version>2.0.0-beta</version>
      <version>2.0.0-RC1</version>
    </versions>
    <lastUpdated>20241215103000</lastUpdated>
  </versioning>
</metadata>"#;

        let metadata = MavenMetadata::parse(xml).unwrap();
        let processed = client.process_metadata(&metadata);

        assert_eq!(processed.latest_stable, Some("1.5.0".to_string()));
        assert_eq!(processed.latest_rc, Some("2.0.0-RC1".to_string()));
        assert_eq!(processed.latest_beta, Some("2.0.0-beta".to_string()));
        assert_eq!(processed.latest_alpha, Some("2.0.0-alpha".to_string()));
        assert_eq!(processed.stable_versions.len(), 3);
    }
}
