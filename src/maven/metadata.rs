use serde::Deserialize;

/// Represents the maven-metadata.xml structure from Maven Central
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MavenMetadata {
    pub group_id: Option<String>,
    pub artifact_id: Option<String>,
    pub versioning: Option<Versioning>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Versioning {
    pub latest: Option<String>,
    pub release: Option<String>,
    pub versions: Option<Versions>,
    pub last_updated: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Versions {
    #[serde(rename = "version", default)]
    pub versions: Vec<String>,
}

impl MavenMetadata {
    /// Parse maven-metadata.xml content
    pub fn parse(xml: &str) -> Result<Self, quick_xml::DeError> {
        quick_xml::de::from_str(xml)
    }

    /// Get all versions from the metadata
    pub fn get_versions(&self) -> Vec<String> {
        self.versioning
            .as_ref()
            .and_then(|v| v.versions.as_ref())
            .map(|v| v.versions.clone())
            .unwrap_or_default()
    }

    /// Get the latest version (as declared in metadata)
    pub fn get_latest(&self) -> Option<&str> {
        self.versioning.as_ref()?.latest.as_deref()
    }

    /// Get the release version (as declared in metadata)
    pub fn get_release(&self) -> Option<&str> {
        self.versioning.as_ref()?.release.as_deref()
    }

    /// Get the last updated timestamp
    pub fn get_last_updated(&self) -> Option<&str> {
        self.versioning.as_ref()?.last_updated.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_metadata() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<metadata>
  <groupId>org.springframework</groupId>
  <artifactId>spring-core</artifactId>
  <versioning>
    <latest>6.2.1</latest>
    <release>6.2.1</release>
    <versions>
      <version>5.3.0</version>
      <version>5.3.1</version>
      <version>6.0.0</version>
      <version>6.1.0</version>
      <version>6.2.0</version>
      <version>6.2.1</version>
    </versions>
    <lastUpdated>20241215103000</lastUpdated>
  </versioning>
</metadata>"#;

        let metadata = MavenMetadata::parse(xml).unwrap();
        assert_eq!(metadata.group_id, Some("org.springframework".to_string()));
        assert_eq!(metadata.artifact_id, Some("spring-core".to_string()));
        assert_eq!(metadata.get_latest(), Some("6.2.1"));
        assert_eq!(metadata.get_release(), Some("6.2.1"));
        assert_eq!(metadata.get_versions().len(), 6);
    }

    #[test]
    fn test_parse_empty_versions() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<metadata>
  <groupId>com.example</groupId>
  <artifactId>test</artifactId>
  <versioning>
    <latest>1.0.0</latest>
  </versioning>
</metadata>"#;

        let metadata = MavenMetadata::parse(xml).unwrap();
        assert_eq!(metadata.get_versions().len(), 0);
        assert_eq!(metadata.get_latest(), Some("1.0.0"));
    }
}
