use std::sync::Arc;

use futures::future::join_all;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo};
use rmcp::{tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler};
use schemars::JsonSchema;
use serde::Deserialize;
use tracing::{error, info, instrument};

use crate::maven::MavenClient;
use crate::models::{MavenCoordinate, MavenVersion, UpdateType, VersionStability};
use crate::tools::responses::*;

/// MCP Service providing Maven Central tools
#[derive(Clone)]
pub struct MavenToolsService {
    client: Arc<MavenClient>,
    tool_router: ToolRouter<Self>,
}

impl MavenToolsService {
    pub fn new() -> Self {
        MavenToolsService {
            client: Arc::new(MavenClient::new()),
            tool_router: Self::tool_router(),
        }
    }
}

impl Default for MavenToolsService {
    fn default() -> Self {
        Self::new()
    }
}

// Implement ServerHandler to provide server info
#[tool_handler]
impl ServerHandler for MavenToolsService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "Maven Central MCP Server - Query Maven Central for dependency versions, \
                 updates, and project health analysis. Supports Maven coordinates in format \
                 'groupId:artifactId' or 'groupId:artifactId:version'."
                    .to_string(),
            ),
        }
    }
}

// Tool parameter structs with JSON Schema derivation for MCP

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetLatestVersionParams {
    /// Maven coordinate in format "groupId:artifactId" (e.g., "org.springframework:spring-core")
    #[schemars(description = "Maven coordinate like 'org.springframework:spring-core'")]
    pub dependency: String,

    /// If true, prioritize stable versions over pre-release versions
    #[schemars(description = "Prioritize stable versions (default: true)")]
    #[serde(default = "default_true")]
    pub prefer_stable: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CheckVersionExistsParams {
    /// Maven coordinate with version in format "groupId:artifactId:version"
    #[schemars(
        description = "Maven coordinate with version like 'org.springframework:spring-core:6.1.0'"
    )]
    pub dependency: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CompareVersionsParams {
    /// Maven coordinate with current version in format "groupId:artifactId:version"
    #[schemars(
        description = "Maven coordinate with version like 'org.springframework:spring-core:5.3.0'"
    )]
    pub dependency: String,

    /// Only compare against stable versions
    #[schemars(description = "Only suggest stable version upgrades (default: true)")]
    #[serde(default = "default_true")]
    pub stable_only: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CheckMultipleDependenciesParams {
    /// List of Maven coordinates (with or without versions)
    #[schemars(
        description = "List of Maven coordinates like ['org.springframework:spring-core:5.3.0', 'com.google.guava:guava:31.0-jre']"
    )]
    pub dependencies: Vec<String>,

    /// Only show stable versions in results
    #[schemars(description = "Only suggest stable version upgrades (default: true)")]
    #[serde(default = "default_true")]
    pub stable_only: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AnalyzeDependencyAgeParams {
    /// Maven coordinate with version in format "groupId:artifactId:version"
    #[schemars(
        description = "Maven coordinate with version like 'org.springframework:spring-core:5.3.0'"
    )]
    pub dependency: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AnalyzeProjectHealthParams {
    /// List of Maven coordinates with versions
    #[schemars(
        description = "List of Maven coordinates with versions like ['org.springframework:spring-core:5.3.0', 'com.fasterxml.jackson.core:jackson-core:2.15.0']"
    )]
    pub dependencies: Vec<String>,
}

fn default_true() -> bool {
    true
}

// Tool implementations

#[tool_router]
impl MavenToolsService {
    /// Get the latest version of a Maven dependency with stability classification
    #[tool(
        name = "get_latest_version",
        description = "Get the latest version of a Maven dependency from Maven Central with stability classification (stable, RC, beta, alpha, milestone)"
    )]
    #[instrument(skip(self))]
    async fn get_latest_version(
        &self,
        params: Parameters<GetLatestVersionParams>,
    ) -> Result<CallToolResult, McpError> {
        info!("get_latest_version: {}", params.0.dependency);

        let coordinate = MavenCoordinate::parse(&params.0.dependency)
            .map_err(|e| McpError::invalid_params(e.to_string(), None))?;

        let metadata = self
            .client
            .get_metadata(&coordinate)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let response = LatestVersionResponse {
            dependency: coordinate.to_ga(),
            latest: LatestVersions {
                stable: metadata.latest_stable.clone(),
                rc: metadata.latest_rc.clone(),
                beta: metadata.latest_beta.clone(),
                alpha: metadata.latest_alpha.clone(),
                milestone: metadata.latest_milestone.clone(),
                any: metadata.latest_any.clone(),
            },
            total_versions: metadata.all_versions.len(),
            stable_versions: metadata.stable_versions.len(),
            last_updated: metadata.last_updated.clone(),
        };

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&response)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?,
        )]))
    }

    /// Check if a specific version exists on Maven Central
    #[tool(
        name = "check_version_exists",
        description = "Verify if a specific version of a Maven dependency exists on Maven Central and get its stability classification"
    )]
    #[instrument(skip(self))]
    async fn check_version_exists(
        &self,
        params: Parameters<CheckVersionExistsParams>,
    ) -> Result<CallToolResult, McpError> {
        info!("check_version_exists: {}", params.0.dependency);

        let coordinate = MavenCoordinate::parse(&params.0.dependency)
            .map_err(|e| McpError::invalid_params(e.to_string(), None))?;

        let version = coordinate
            .version
            .clone()
            .ok_or_else(|| McpError::invalid_params("Version is required", None))?;

        let metadata = self
            .client
            .get_metadata(&coordinate)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let exists = metadata.all_versions.iter().any(|v| v == &version);

        let response = VersionExistsResponse {
            dependency: coordinate.to_ga(),
            version: version.clone(),
            exists,
            stability: if exists {
                Some(VersionStability::classify(&version))
            } else {
                None
            },
            latest_stable: metadata.latest_stable.clone(),
        };

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&response)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?,
        )]))
    }

    /// Compare current version against latest and determine update type
    #[tool(
        name = "compare_versions",
        description = "Compare your current version against the latest version, determine if you're outdated, and classify the update type (major/minor/patch)"
    )]
    #[instrument(skip(self))]
    async fn compare_versions(
        &self,
        params: Parameters<CompareVersionsParams>,
    ) -> Result<CallToolResult, McpError> {
        info!("compare_versions: {}", params.0.dependency);

        let coordinate = MavenCoordinate::parse(&params.0.dependency)
            .map_err(|e| McpError::invalid_params(e.to_string(), None))?;

        let current_version = coordinate
            .version
            .clone()
            .ok_or_else(|| McpError::invalid_params("Version is required", None))?;

        let metadata = self
            .client
            .get_metadata(&coordinate)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let target_version = if params.0.stable_only {
            metadata.latest_stable.clone()
        } else {
            metadata.latest_any.clone()
        };

        let target_version = target_version
            .ok_or_else(|| McpError::internal_error("No versions found", None))?;

        let current_parsed = MavenVersion::parse(&current_version);
        let target_parsed = MavenVersion::parse(&target_version);

        let is_outdated = current_parsed < target_parsed;
        let update_type = if is_outdated {
            Some(UpdateType::between(&current_version, &target_version))
        } else {
            None
        };

        // Count versions between current and latest
        let versions_behind = metadata
            .all_versions
            .iter()
            .filter(|v| {
                let parsed = MavenVersion::parse(v);
                parsed > current_parsed && parsed <= target_parsed
            })
            .count();

        let recommendation = if is_outdated {
            Some(format!(
                "Consider upgrading from {} to {} ({} update, {} versions behind)",
                current_version,
                target_version,
                update_type.unwrap_or(UpdateType::Other),
                versions_behind
            ))
        } else {
            Some("You're using the latest version!".to_string())
        };

        let response = VersionComparisonResponse {
            dependency: coordinate.to_ga(),
            current_version: current_version.clone(),
            current_stability: VersionStability::classify(&current_version),
            latest_version: target_version.clone(),
            latest_stability: VersionStability::classify(&target_version),
            is_outdated,
            update_type,
            versions_behind,
            recommendation,
        };

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&response)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?,
        )]))
    }

    /// Check multiple dependencies for updates in bulk
    #[tool(
        name = "check_multiple_dependencies",
        description = "Bulk check multiple Maven dependencies for available updates. Efficient for analyzing entire projects."
    )]
    #[instrument(skip(self))]
    async fn check_multiple_dependencies(
        &self,
        params: Parameters<CheckMultipleDependenciesParams>,
    ) -> Result<CallToolResult, McpError> {
        info!(
            "check_multiple_dependencies: {} dependencies",
            params.0.dependencies.len()
        );

        let client = self.client.clone();
        let stable_only = params.0.stable_only;

        // Process all dependencies concurrently
        let futures: Vec<_> = params
            .0
            .dependencies
            .iter()
            .map(|dep| {
                let client = client.clone();
                let dep = dep.clone();
                async move { check_single_dependency(&client, &dep, stable_only).await }
            })
            .collect();

        let results = join_all(futures).await;

        // Aggregate results
        let mut outdated_count = 0;
        let mut up_to_date_count = 0;
        let mut error_count = 0;
        let mut major_updates = 0;
        let mut minor_updates = 0;
        let mut patch_updates = 0;

        for result in &results {
            if result.error.is_some() {
                error_count += 1;
            } else if result.is_outdated {
                outdated_count += 1;
                match result.update_type {
                    Some(UpdateType::Major) => major_updates += 1,
                    Some(UpdateType::Minor) => minor_updates += 1,
                    Some(UpdateType::Patch) => patch_updates += 1,
                    _ => {}
                }
            } else {
                up_to_date_count += 1;
            }
        }

        let response = BulkCheckResponse {
            total_checked: params.0.dependencies.len(),
            outdated_count,
            up_to_date_count,
            error_count,
            dependencies: results,
            summary: BulkCheckSummary {
                major_updates,
                minor_updates,
                patch_updates,
            },
        };

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&response)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?,
        )]))
    }

    /// Analyze how outdated a dependency version is
    #[tool(
        name = "analyze_dependency_age",
        description = "Analyze how outdated a dependency is, classifying it as current, fresh, aging, stale, or outdated based on versions behind"
    )]
    #[instrument(skip(self))]
    async fn analyze_dependency_age(
        &self,
        params: Parameters<AnalyzeDependencyAgeParams>,
    ) -> Result<CallToolResult, McpError> {
        info!("analyze_dependency_age: {}", params.0.dependency);

        let coordinate = MavenCoordinate::parse(&params.0.dependency)
            .map_err(|e| McpError::invalid_params(e.to_string(), None))?;

        let current_version = coordinate
            .version
            .clone()
            .ok_or_else(|| McpError::invalid_params("Version is required", None))?;

        let metadata = self
            .client
            .get_metadata(&coordinate)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let current_parsed = MavenVersion::parse(&current_version);

        // Count total versions newer than current
        let versions_since = metadata
            .all_versions
            .iter()
            .filter(|v| MavenVersion::parse(v) > current_parsed)
            .count();

        // Count stable versions newer than current
        let stable_versions_since = metadata
            .stable_versions
            .iter()
            .filter(|v| MavenVersion::parse(v) > current_parsed)
            .count();

        // Determine age classification based on stable versions behind
        let age_classification = if stable_versions_since == 0 {
            AgeClassification::Current
        } else if stable_versions_since <= 2 {
            AgeClassification::Fresh
        } else if stable_versions_since <= 5 {
            AgeClassification::Aging
        } else {
            // Check if it's a major version behind
            if let Some(latest) = &metadata.latest_stable {
                let update_type = UpdateType::between(&current_version, latest);
                if update_type == UpdateType::Major {
                    AgeClassification::Outdated
                } else {
                    AgeClassification::Stale
                }
            } else {
                AgeClassification::Stale
            }
        };

        let recommendation = match age_classification {
            AgeClassification::Current => None,
            AgeClassification::Fresh => Some(
                "Minor updates available. Consider upgrading when convenient.".to_string(),
            ),
            AgeClassification::Aging => Some(format!(
                "Several versions behind ({}). Plan an upgrade soon.",
                stable_versions_since
            )),
            AgeClassification::Stale => Some(format!(
                "Significantly outdated ({} versions behind). Prioritize upgrading.",
                stable_versions_since
            )),
            AgeClassification::Outdated => {
                Some("Major version behind! Upgrade may require code changes.".to_string())
            }
        };

        let response = DependencyAgeResponse {
            dependency: coordinate.to_ga(),
            version: current_version,
            age_classification,
            versions_since,
            stable_versions_since,
            latest_stable: metadata.latest_stable.clone(),
            recommendation,
        };

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&response)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?,
        )]))
    }

    /// Analyze overall project dependency health
    #[tool(
        name = "analyze_project_health",
        description = "Comprehensive health analysis of all project dependencies with overall health score and grade (A-F)"
    )]
    #[instrument(skip(self))]
    async fn analyze_project_health(
        &self,
        params: Parameters<AnalyzeProjectHealthParams>,
    ) -> Result<CallToolResult, McpError> {
        info!(
            "analyze_project_health: {} dependencies",
            params.0.dependencies.len()
        );

        let client = self.client.clone();

        // Process all dependencies concurrently
        let futures: Vec<_> = params
            .0
            .dependencies
            .iter()
            .map(|dep| {
                let client = client.clone();
                let dep = dep.clone();
                async move { analyze_single_health(&client, &dep).await }
            })
            .collect();

        let results = join_all(futures).await;

        // Calculate summary
        let mut summary = HealthSummary {
            current: 0,
            fresh: 0,
            aging: 0,
            stale: 0,
            outdated: 0,
            errors: 0,
        };

        let mut total_score = 0.0;
        let mut scored_count = 0;

        for result in &results {
            if result.error.is_some() {
                summary.errors += 1;
            } else if let Some(age) = result.age_classification {
                scored_count += 1;
                total_score += result.health_score;
                match age {
                    AgeClassification::Current => summary.current += 1,
                    AgeClassification::Fresh => summary.fresh += 1,
                    AgeClassification::Aging => summary.aging += 1,
                    AgeClassification::Stale => summary.stale += 1,
                    AgeClassification::Outdated => summary.outdated += 1,
                }
            }
        }

        let health_score = if scored_count > 0 {
            total_score / scored_count as f32
        } else {
            0.0
        };

        let health_grade = HealthGrade::from_score(health_score);

        // Generate recommendations
        let mut recommendations = Vec::new();
        if summary.outdated > 0 {
            recommendations.push(format!(
                "{} dependencies are a major version behind. Prioritize these upgrades.",
                summary.outdated
            ));
        }
        if summary.stale > 0 {
            recommendations.push(format!(
                "{} dependencies are significantly outdated. Plan upgrades soon.",
                summary.stale
            ));
        }
        if summary.aging > 0 {
            recommendations.push(format!(
                "{} dependencies have updates available. Consider upgrading.",
                summary.aging
            ));
        }
        if summary.current + summary.fresh == params.0.dependencies.len() - summary.errors {
            recommendations.push("All dependencies are up to date!".to_string());
        }

        let response = ProjectHealthResponse {
            total_dependencies: params.0.dependencies.len(),
            health_score,
            health_grade,
            summary,
            dependencies: results,
            recommendations,
        };

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&response)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?,
        )]))
    }
}

/// Helper function to check a single dependency
async fn check_single_dependency(
    client: &MavenClient,
    dependency: &str,
    stable_only: bool,
) -> DependencyCheckResult {
    let coordinate = match MavenCoordinate::parse(dependency) {
        Ok(c) => c,
        Err(e) => {
            return DependencyCheckResult {
                dependency: dependency.to_string(),
                current_version: None,
                latest_version: None,
                is_outdated: false,
                update_type: None,
                error: Some(e.to_string()),
            }
        }
    };

    let current_version = coordinate.version.clone();

    let metadata = match client.get_metadata(&coordinate).await {
        Ok(m) => m,
        Err(e) => {
            error!("Failed to fetch metadata for {}: {}", dependency, e);
            return DependencyCheckResult {
                dependency: coordinate.to_ga(),
                current_version,
                latest_version: None,
                is_outdated: false,
                update_type: None,
                error: Some(e.to_string()),
            };
        }
    };

    let latest_version = if stable_only {
        metadata.latest_stable.clone()
    } else {
        metadata.latest_any.clone()
    };

    match (&current_version, &latest_version) {
        (Some(current), Some(latest)) => {
            let current_parsed = MavenVersion::parse(current);
            let latest_parsed = MavenVersion::parse(latest);
            let is_outdated = current_parsed < latest_parsed;
            let update_type = if is_outdated {
                Some(UpdateType::between(current, latest))
            } else {
                None
            };

            DependencyCheckResult {
                dependency: coordinate.to_ga(),
                current_version: Some(current.clone()),
                latest_version: Some(latest.clone()),
                is_outdated,
                update_type,
                error: None,
            }
        }
        (None, Some(latest)) => {
            // No current version specified, just return latest
            DependencyCheckResult {
                dependency: coordinate.to_ga(),
                current_version: None,
                latest_version: Some(latest.clone()),
                is_outdated: false,
                update_type: None,
                error: None,
            }
        }
        _ => DependencyCheckResult {
            dependency: coordinate.to_ga(),
            current_version,
            latest_version: None,
            is_outdated: false,
            update_type: None,
            error: Some("No versions found".to_string()),
        },
    }
}

/// Helper function to analyze health of a single dependency
async fn analyze_single_health(client: &MavenClient, dependency: &str) -> DependencyHealthResult {
    let coordinate = match MavenCoordinate::parse(dependency) {
        Ok(c) => c,
        Err(e) => {
            return DependencyHealthResult {
                dependency: dependency.to_string(),
                current_version: None,
                latest_version: None,
                age_classification: None,
                health_score: 0.0,
                update_type: None,
                error: Some(e.to_string()),
            }
        }
    };

    let current_version = match &coordinate.version {
        Some(v) => v.clone(),
        None => {
            return DependencyHealthResult {
                dependency: coordinate.to_ga(),
                current_version: None,
                latest_version: None,
                age_classification: None,
                health_score: 0.0,
                update_type: None,
                error: Some("Version is required for health analysis".to_string()),
            }
        }
    };

    let metadata = match client.get_metadata(&coordinate).await {
        Ok(m) => m,
        Err(e) => {
            error!("Failed to fetch metadata for {}: {}", dependency, e);
            return DependencyHealthResult {
                dependency: coordinate.to_ga(),
                current_version: Some(current_version),
                latest_version: None,
                age_classification: None,
                health_score: 0.0,
                update_type: None,
                error: Some(e.to_string()),
            };
        }
    };

    let current_parsed = MavenVersion::parse(&current_version);

    // Count stable versions newer than current
    let stable_versions_since = metadata
        .stable_versions
        .iter()
        .filter(|v| MavenVersion::parse(v) > current_parsed)
        .count();

    // Calculate health score and age classification
    let (age_classification, health_score) = if stable_versions_since == 0 {
        (AgeClassification::Current, 100.0)
    } else if stable_versions_since <= 2 {
        (AgeClassification::Fresh, 90.0)
    } else if stable_versions_since <= 5 {
        (AgeClassification::Aging, 70.0)
    } else {
        // Check for major version difference
        if let Some(latest) = &metadata.latest_stable {
            let update_type = UpdateType::between(&current_version, latest);
            if update_type == UpdateType::Major {
                (AgeClassification::Outdated, 40.0)
            } else {
                (AgeClassification::Stale, 50.0)
            }
        } else {
            (AgeClassification::Stale, 50.0)
        }
    };

    let update_type = metadata
        .latest_stable
        .as_ref()
        .map(|latest| UpdateType::between(&current_version, latest));

    DependencyHealthResult {
        dependency: coordinate.to_ga(),
        current_version: Some(current_version),
        latest_version: metadata.latest_stable.clone(),
        age_classification: Some(age_classification),
        health_score,
        update_type,
        error: None,
    }
}
