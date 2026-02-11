use anyhow::Result;
use regex::Regex;

/// Validate a semantic version string
pub fn validate_semver(version: &str) -> Result<()> {
    let re = Regex::new(r"^\d+\.\d+\.\d+(?:-[a-zA-Z0-9.-]+)?$")?;

    if !re.is_match(version) {
        anyhow::bail!(
            "Invalid semantic version: {}. Expected format: major.minor.patch[-prerelease]",
            version
        );
    }

    Ok(())
}

/// Parse a semantic version into components
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemVer {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub prerelease: Option<String>,
}

impl SemVer {
    pub fn parse(version: &str) -> Result<Self> {
        validate_semver(version)?;

        let (base, prerelease) = if let Some(idx) = version.find('-') {
            (&version[..idx], Some(version[idx + 1..].to_string()))
        } else {
            (version, None)
        };

        let parts: Vec<&str> = base.split('.').collect();
        if parts.len() != 3 {
            anyhow::bail!("Invalid version format: {}", version);
        }

        Ok(SemVer {
            major: parts[0].parse()?,
            minor: parts[1].parse()?,
            patch: parts[2].parse()?,
            prerelease,
        })
    }

    pub fn to_string(&self) -> String {
        if let Some(ref prerelease) = self.prerelease {
            format!(
                "{}.{}.{}-{}",
                self.major, self.minor, self.patch, prerelease
            )
        } else {
            format!("{}.{}.{}", self.major, self.minor, self.patch)
        }
    }
}

impl PartialOrd for SemVer {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SemVer {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.major.cmp(&other.major) {
            std::cmp::Ordering::Equal => match self.minor.cmp(&other.minor) {
                std::cmp::Ordering::Equal => match self.patch.cmp(&other.patch) {
                    std::cmp::Ordering::Equal => {
                        // Prerelease versions have lower precedence
                        match (&self.prerelease, &other.prerelease) {
                            (None, None) => std::cmp::Ordering::Equal,
                            (None, Some(_)) => std::cmp::Ordering::Greater,
                            (Some(_), None) => std::cmp::Ordering::Less,
                            (Some(a), Some(b)) => a.cmp(b),
                        }
                    }
                    ord => ord,
                },
                ord => ord,
            },
            ord => ord,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_semver() {
        assert!(validate_semver("1.0.0").is_ok());
        assert!(validate_semver("1.2.3").is_ok());
        assert!(validate_semver("1.2.3-beta.1").is_ok());
        assert!(validate_semver("1.2.3-alpha").is_ok());
        assert!(validate_semver("1.2").is_err());
        assert!(validate_semver("1").is_err());
        assert!(validate_semver("invalid").is_err());
    }

    #[test]
    fn test_semver_parse() {
        let v = SemVer::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
        assert_eq!(v.prerelease, None);

        let v = SemVer::parse("1.2.3-beta.1").unwrap();
        assert_eq!(v.prerelease, Some("beta.1".to_string()));
    }

    #[test]
    fn test_semver_ordering() {
        let v1 = SemVer::parse("1.0.0").unwrap();
        let v2 = SemVer::parse("1.0.1").unwrap();
        let v3 = SemVer::parse("1.1.0").unwrap();
        let v4 = SemVer::parse("2.0.0").unwrap();
        let v5 = SemVer::parse("1.0.0-beta").unwrap();

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v3 < v4);
        assert!(v5 < v1); // Prerelease < release
    }
}
