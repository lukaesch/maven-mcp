use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoordinateError {
    #[error("Invalid Maven coordinate format: {0}. Expected 'groupId:artifactId' or 'groupId:artifactId:version'")]
    InvalidFormat(String),
    #[error("Empty group ID")]
    EmptyGroupId,
    #[error("Empty artifact ID")]
    EmptyArtifactId,
}

/// Represents a Maven coordinate (GAV - GroupId, ArtifactId, Version)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MavenCoordinate {
    pub group_id: String,
    pub artifact_id: String,
    pub version: Option<String>,
}

impl MavenCoordinate {
    /// Parse a Maven coordinate string
    /// Accepts formats:
    /// - "groupId:artifactId"
    /// - "groupId:artifactId:version"
    pub fn parse(input: &str) -> Result<Self, CoordinateError> {
        let input = input.trim();
        let parts: Vec<&str> = input.split(':').collect();

        match parts.len() {
            2 => {
                let group_id = parts[0].trim();
                let artifact_id = parts[1].trim();

                if group_id.is_empty() {
                    return Err(CoordinateError::EmptyGroupId);
                }
                if artifact_id.is_empty() {
                    return Err(CoordinateError::EmptyArtifactId);
                }

                Ok(MavenCoordinate {
                    group_id: group_id.to_string(),
                    artifact_id: artifact_id.to_string(),
                    version: None,
                })
            }
            3 | 4 | 5 => {
                // 3 = g:a:v, 4 = g:a:packaging:v, 5 = g:a:packaging:classifier:v
                let group_id = parts[0].trim();
                let artifact_id = parts[1].trim();
                let version = parts.last().unwrap().trim();

                if group_id.is_empty() {
                    return Err(CoordinateError::EmptyGroupId);
                }
                if artifact_id.is_empty() {
                    return Err(CoordinateError::EmptyArtifactId);
                }

                let version = if version.is_empty() {
                    None
                } else {
                    Some(version.to_string())
                };

                Ok(MavenCoordinate {
                    group_id: group_id.to_string(),
                    artifact_id: artifact_id.to_string(),
                    version,
                })
            }
            _ => Err(CoordinateError::InvalidFormat(input.to_string())),
        }
    }

    /// Returns the path segment for Maven repository URLs
    /// e.g., "org.springframework" -> "org/springframework"
    pub fn group_path(&self) -> String {
        self.group_id.replace('.', "/")
    }

    /// Returns the full repository path for maven-metadata.xml
    /// e.g., "org/springframework/spring-core"
    pub fn metadata_path(&self) -> String {
        format!("{}/{}", self.group_path(), self.artifact_id)
    }

    /// Returns coordinate without version as "groupId:artifactId"
    pub fn to_ga(&self) -> String {
        format!("{}:{}", self.group_id, self.artifact_id)
    }

    /// Returns full coordinate as "groupId:artifactId:version" if version exists
    pub fn to_gav(&self) -> Option<String> {
        self.version
            .as_ref()
            .map(|v| format!("{}:{}:{}", self.group_id, self.artifact_id, v))
    }
}

impl fmt::Display for MavenCoordinate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.version {
            Some(v) => write!(f, "{}:{}:{}", self.group_id, self.artifact_id, v),
            None => write!(f, "{}:{}", self.group_id, self.artifact_id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ga() {
        let coord = MavenCoordinate::parse("org.springframework:spring-core").unwrap();
        assert_eq!(coord.group_id, "org.springframework");
        assert_eq!(coord.artifact_id, "spring-core");
        assert_eq!(coord.version, None);
    }

    #[test]
    fn test_parse_gav() {
        let coord = MavenCoordinate::parse("org.springframework:spring-core:6.1.0").unwrap();
        assert_eq!(coord.group_id, "org.springframework");
        assert_eq!(coord.artifact_id, "spring-core");
        assert_eq!(coord.version, Some("6.1.0".to_string()));
    }

    #[test]
    fn test_parse_with_packaging() {
        let coord = MavenCoordinate::parse("org.springframework:spring-core:jar:6.1.0").unwrap();
        assert_eq!(coord.group_id, "org.springframework");
        assert_eq!(coord.artifact_id, "spring-core");
        assert_eq!(coord.version, Some("6.1.0".to_string()));
    }

    #[test]
    fn test_group_path() {
        let coord = MavenCoordinate::parse("org.springframework:spring-core").unwrap();
        assert_eq!(coord.group_path(), "org/springframework");
    }

    #[test]
    fn test_metadata_path() {
        let coord = MavenCoordinate::parse("org.springframework:spring-core").unwrap();
        assert_eq!(coord.metadata_path(), "org/springframework/spring-core");
    }

    #[test]
    fn test_invalid_format() {
        assert!(MavenCoordinate::parse("invalid").is_err());
        assert!(MavenCoordinate::parse("").is_err());
        assert!(MavenCoordinate::parse(":artifact").is_err());
        assert!(MavenCoordinate::parse("group:").is_err());
    }
}
