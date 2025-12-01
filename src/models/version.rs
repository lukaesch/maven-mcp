use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Classification of version stability
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VersionStability {
    Stable,
    #[serde(rename = "rc")]
    RC,
    Beta,
    Alpha,
    Milestone,
    Snapshot,
}

impl VersionStability {
    /// Classify a version string into a stability category
    pub fn classify(version: &str) -> Self {
        let v = version.to_uppercase();

        if v.contains("SNAPSHOT") {
            return VersionStability::Snapshot;
        }
        if v.contains("ALPHA") || v.contains("-A.") || v.ends_with("-A") {
            return VersionStability::Alpha;
        }
        if v.contains("BETA") || v.contains("-B.") || v.ends_with("-B") {
            return VersionStability::Beta;
        }
        if v.contains("-RC") || v.contains(".RC") || v.contains("-CR") || v.contains(".CR") {
            return VersionStability::RC;
        }
        if v.contains("-M") || v.contains(".M") {
            // Check if it's actually a milestone (M followed by number)
            let has_milestone = v
                .split(|c: char| !c.is_alphanumeric())
                .any(|part| {
                    part.starts_with('M') && part.len() > 1 && part[1..].chars().all(|c| c.is_ascii_digit())
                });
            if has_milestone {
                return VersionStability::Milestone;
            }
        }

        VersionStability::Stable
    }

    /// Returns true if this stability level is considered production-ready
    pub fn is_stable(&self) -> bool {
        matches!(self, VersionStability::Stable)
    }

    /// Priority for sorting (higher = more stable)
    pub fn priority(&self) -> u8 {
        match self {
            VersionStability::Stable => 6,
            VersionStability::RC => 5,
            VersionStability::Beta => 4,
            VersionStability::Alpha => 3,
            VersionStability::Milestone => 2,
            VersionStability::Snapshot => 1,
        }
    }
}

impl std::fmt::Display for VersionStability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VersionStability::Stable => write!(f, "stable"),
            VersionStability::RC => write!(f, "rc"),
            VersionStability::Beta => write!(f, "beta"),
            VersionStability::Alpha => write!(f, "alpha"),
            VersionStability::Milestone => write!(f, "milestone"),
            VersionStability::Snapshot => write!(f, "snapshot"),
        }
    }
}

/// A parsed Maven version with comparison capabilities
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MavenVersion {
    pub original: String,
    pub stability: VersionStability,
    parts: Vec<VersionPart>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum VersionPart {
    Number(u64),
    String(String),
}

impl MavenVersion {
    pub fn parse(version: &str) -> Self {
        let stability = VersionStability::classify(version);
        let parts = Self::parse_parts(version);

        MavenVersion {
            original: version.to_string(),
            stability,
            parts,
        }
    }

    fn parse_parts(version: &str) -> Vec<VersionPart> {
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut in_number = false;

        for c in version.chars() {
            if c.is_ascii_digit() {
                if !in_number && !current.is_empty() {
                    parts.push(VersionPart::String(current.to_lowercase()));
                    current.clear();
                }
                in_number = true;
                current.push(c);
            } else if c == '.' || c == '-' || c == '_' {
                if !current.is_empty() {
                    if in_number {
                        parts.push(VersionPart::Number(current.parse().unwrap_or(0)));
                    } else {
                        parts.push(VersionPart::String(current.to_lowercase()));
                    }
                    current.clear();
                }
                in_number = false;
            } else {
                if in_number && !current.is_empty() {
                    parts.push(VersionPart::Number(current.parse().unwrap_or(0)));
                    current.clear();
                }
                in_number = false;
                current.push(c);
            }
        }

        if !current.is_empty() {
            if in_number {
                parts.push(VersionPart::Number(current.parse().unwrap_or(0)));
            } else {
                parts.push(VersionPart::String(current.to_lowercase()));
            }
        }

        parts
    }

    /// Compare two versions, returning the relationship
    pub fn compare(&self, other: &MavenVersion) -> Ordering {
        // Compare version parts
        let max_len = self.parts.len().max(other.parts.len());

        for i in 0..max_len {
            let self_part = self.parts.get(i);
            let other_part = other.parts.get(i);

            match (self_part, other_part) {
                (Some(VersionPart::Number(a)), Some(VersionPart::Number(b))) => {
                    match a.cmp(b) {
                        Ordering::Equal => continue,
                        ord => return ord,
                    }
                }
                (Some(VersionPart::String(a)), Some(VersionPart::String(b))) => {
                    // Handle qualifier ordering
                    let ord = compare_qualifiers(a, b);
                    if ord != Ordering::Equal {
                        return ord;
                    }
                }
                (Some(VersionPart::Number(_)), Some(VersionPart::String(s))) => {
                    // Numbers come after qualifiers (1.0 > 1.0-alpha)
                    if is_qualifier(s) {
                        return Ordering::Greater;
                    }
                    return Ordering::Less;
                }
                (Some(VersionPart::String(s)), Some(VersionPart::Number(_))) => {
                    if is_qualifier(s) {
                        return Ordering::Less;
                    }
                    return Ordering::Greater;
                }
                (Some(_), None) => {
                    // Having more parts usually means newer, unless it's a qualifier
                    if let Some(VersionPart::String(s)) = self_part {
                        if is_qualifier(s) {
                            return Ordering::Less;
                        }
                    }
                    return Ordering::Greater;
                }
                (None, Some(_)) => {
                    if let Some(VersionPart::String(s)) = other_part {
                        if is_qualifier(s) {
                            return Ordering::Greater;
                        }
                    }
                    return Ordering::Less;
                }
                (None, None) => break,
            }
        }

        Ordering::Equal
    }
}

fn is_qualifier(s: &str) -> bool {
    matches!(
        s.to_lowercase().as_str(),
        "alpha" | "beta" | "rc" | "cr" | "snapshot" | "m" | "milestone" | "a" | "b"
    )
}

fn compare_qualifiers(a: &str, b: &str) -> Ordering {
    let priority = |s: &str| -> i32 {
        match s.to_lowercase().as_str() {
            "snapshot" => 0,
            "alpha" | "a" => 1,
            "beta" | "b" => 2,
            "milestone" | "m" => 3,
            "rc" | "cr" => 4,
            "ga" | "final" | "release" => 5,
            _ => 3, // Unknown qualifiers treated as milestone-level
        }
    };

    priority(a).cmp(&priority(b))
}

impl Ord for MavenVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        self.compare(other)
    }
}

impl PartialOrd for MavenVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Determines the type of update between two versions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateType {
    Major,
    Minor,
    Patch,
    Other,
}

impl UpdateType {
    /// Determine update type between current and target version
    pub fn between(current: &str, target: &str) -> Self {
        let current_parts = Self::extract_numeric_parts(current);
        let target_parts = Self::extract_numeric_parts(target);

        if current_parts.is_empty() || target_parts.is_empty() {
            return UpdateType::Other;
        }

        let current_major = current_parts.first().copied().unwrap_or(0);
        let target_major = target_parts.first().copied().unwrap_or(0);

        if target_major != current_major {
            return UpdateType::Major;
        }

        let current_minor = current_parts.get(1).copied().unwrap_or(0);
        let target_minor = target_parts.get(1).copied().unwrap_or(0);

        if target_minor != current_minor {
            return UpdateType::Minor;
        }

        UpdateType::Patch
    }

    fn extract_numeric_parts(version: &str) -> Vec<u64> {
        version
            .split(|c: char| !c.is_ascii_digit())
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect()
    }
}

impl std::fmt::Display for UpdateType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateType::Major => write!(f, "major"),
            UpdateType::Minor => write!(f, "minor"),
            UpdateType::Patch => write!(f, "patch"),
            UpdateType::Other => write!(f, "other"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stability_classification() {
        assert_eq!(VersionStability::classify("1.0.0"), VersionStability::Stable);
        assert_eq!(VersionStability::classify("6.2.1"), VersionStability::Stable);
        assert_eq!(VersionStability::classify("1.0.0-SNAPSHOT"), VersionStability::Snapshot);
        assert_eq!(VersionStability::classify("1.0.0-alpha"), VersionStability::Alpha);
        assert_eq!(VersionStability::classify("1.0.0-alpha.1"), VersionStability::Alpha);
        assert_eq!(VersionStability::classify("1.0.0-beta"), VersionStability::Beta);
        assert_eq!(VersionStability::classify("1.0.0-beta1"), VersionStability::Beta);
        assert_eq!(VersionStability::classify("1.0.0-RC1"), VersionStability::RC);
        assert_eq!(VersionStability::classify("1.0.0-rc.1"), VersionStability::RC);
        assert_eq!(VersionStability::classify("1.0.0-M1"), VersionStability::Milestone);
        assert_eq!(VersionStability::classify("1.0.0.M2"), VersionStability::Milestone);
    }

    #[test]
    fn test_version_comparison() {
        let v1 = MavenVersion::parse("1.0.0");
        let v2 = MavenVersion::parse("2.0.0");
        assert!(v1 < v2);

        let v1 = MavenVersion::parse("1.0.0");
        let v2 = MavenVersion::parse("1.1.0");
        assert!(v1 < v2);

        let v1 = MavenVersion::parse("1.0.0");
        let v2 = MavenVersion::parse("1.0.1");
        assert!(v1 < v2);

        let v1 = MavenVersion::parse("1.0.0-alpha");
        let v2 = MavenVersion::parse("1.0.0");
        assert!(v1 < v2);

        let v1 = MavenVersion::parse("1.0.0-RC1");
        let v2 = MavenVersion::parse("1.0.0");
        assert!(v1 < v2);
    }

    #[test]
    fn test_update_type() {
        assert_eq!(UpdateType::between("1.0.0", "2.0.0"), UpdateType::Major);
        assert_eq!(UpdateType::between("1.0.0", "1.1.0"), UpdateType::Minor);
        assert_eq!(UpdateType::between("1.0.0", "1.0.1"), UpdateType::Patch);
        assert_eq!(UpdateType::between("1.0.0", "1.0.0"), UpdateType::Patch);
    }
}
