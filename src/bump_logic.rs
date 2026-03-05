use std::fmt;

use crate::commit_parser::CommitImpact;

#[derive(Debug, Clone, PartialEq)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub dev: Option<u64>,
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.dev {
            None => write!(f, "{}.{}.{}", self.major, self.minor, self.patch),
            Some(n) => write!(f, "{}.{}.{}-dev{}", self.major, self.minor, self.patch, n),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum NextDevTarget {
    Patch,
    Minor,
}

impl NextDevTarget {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "patch" => Ok(NextDevTarget::Patch),
            "minor" => Ok(NextDevTarget::Minor),
            other => Err(format!("Invalid next-dev-target: '{}'", other)),
        }
    }
}

pub fn parse_version(s: &str) -> Result<Version, String> {
    // Split on '-' to separate base from pre-release
    let (base, dev) = if let Some(idx) = s.find('-') {
        (&s[..idx], Some(&s[idx + 1..]))
    } else {
        (s, None)
    };

    let parts: Vec<&str> = base.split('.').collect();
    if parts.len() != 3 {
        return Err(format!("Invalid version format (expected X.Y.Z): '{}'", s));
    }

    let major = parts[0]
        .parse::<u64>()
        .map_err(|_| format!("Invalid major version in '{}'", s))?;
    let minor = parts[1]
        .parse::<u64>()
        .map_err(|_| format!("Invalid minor version in '{}'", s))?;
    let patch = parts[2]
        .parse::<u64>()
        .map_err(|_| format!("Invalid patch version in '{}'", s))?;

    let dev_num = match dev {
        None => None,
        Some(pre) => {
            if let Some(n_str) = pre.strip_prefix("dev") {
                let n = n_str
                    .parse::<u64>()
                    .map_err(|_| format!("Invalid dev counter in '{}'", s))?;
                Some(n)
            } else {
                return Err(format!(
                    "Unsupported pre-release format '{}' in version '{}'",
                    pre, s
                ));
            }
        }
    };

    Ok(Version {
        major,
        minor,
        patch,
        dev: dev_num,
    })
}

fn dev_bump(v: &Version) -> Version {
    Version {
        major: v.major,
        minor: v.minor,
        patch: v.patch,
        dev: Some(match v.dev {
            None => 0,
            Some(n) => n + 1,
        }),
    }
}

fn minor_bump_or_dev(current: &Version) -> Version {
    if current.patch == 0 {
        dev_bump(current)
    } else {
        Version {
            major: current.major,
            minor: current.minor + 1,
            patch: 0,
            dev: Some(0),
        }
    }
}

pub fn compute_bump(current: &Version, impact: CommitImpact) -> Version {
    match impact {
        CommitImpact::Breaking | CommitImpact::Refactor => {
            if current.major == 0 {
                // Downgrade major bump to minor-level when major == 0
                minor_bump_or_dev(current)
            } else if current.minor == 0 && current.patch == 0 {
                // Already at x.0.0 -> dev bump only
                dev_bump(current)
            } else {
                // Major bump
                Version {
                    major: current.major + 1,
                    minor: 0,
                    patch: 0,
                    dev: Some(0),
                }
            }
        }
        CommitImpact::Feature => minor_bump_or_dev(current),
        CommitImpact::Dev => dev_bump(current),
    }
}

pub fn next_dev_version(stable: &Version, target: NextDevTarget) -> Version {
    match target {
        NextDevTarget::Patch => Version {
            major: stable.major,
            minor: stable.minor,
            patch: stable.patch + 1,
            dev: Some(0),
        },
        NextDevTarget::Minor => Version {
            major: stable.major,
            minor: stable.minor + 1,
            patch: 0,
            dev: Some(0),
        },
    }
}

pub fn strip_dev_suffix(v: &Version) -> Version {
    Version {
        major: v.major,
        minor: v.minor,
        patch: v.patch,
        dev: None,
    }
}

pub fn is_stable(v: &Version) -> bool {
    v.dev.is_none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commit_parser::CommitImpact;

    #[test]
    fn test_dev_bump_no_suffix() {
        let v = parse_version("0.1.0").unwrap();
        assert_eq!(compute_bump(&v, CommitImpact::Dev).to_string(), "0.1.0-dev0");
    }

    #[test]
    fn test_dev_bump_with_suffix() {
        let v = parse_version("1.2.3-dev5").unwrap();
        assert_eq!(compute_bump(&v, CommitImpact::Dev).to_string(), "1.2.3-dev6");
    }

    #[test]
    fn test_feat_major_zero_patch_gt0() {
        let v = parse_version("0.1.3-dev2").unwrap();
        assert_eq!(
            compute_bump(&v, CommitImpact::Feature).to_string(),
            "0.2.0-dev0"
        );
    }

    #[test]
    fn test_feat_major_zero_patch_zero() {
        let v = parse_version("0.1.0-dev2").unwrap();
        assert_eq!(
            compute_bump(&v, CommitImpact::Feature).to_string(),
            "0.1.0-dev3"
        );
    }

    #[test]
    fn test_feat_patch_zero() {
        let v = parse_version("1.2.0-dev3").unwrap();
        assert_eq!(
            compute_bump(&v, CommitImpact::Feature).to_string(),
            "1.2.0-dev4"
        );
    }

    #[test]
    fn test_feat_minor_bump() {
        let v = parse_version("1.2.5-dev3").unwrap();
        assert_eq!(
            compute_bump(&v, CommitImpact::Feature).to_string(),
            "1.3.0-dev0"
        );
    }

    #[test]
    fn test_breaking_major_bump() {
        let v = parse_version("1.5.2-dev1").unwrap();
        assert_eq!(
            compute_bump(&v, CommitImpact::Breaking).to_string(),
            "2.0.0-dev0"
        );
    }

    #[test]
    fn test_refactor_major_bump() {
        let v = parse_version("1.5.2-dev1").unwrap();
        assert_eq!(
            compute_bump(&v, CommitImpact::Refactor).to_string(),
            "2.0.0-dev0"
        );
    }

    #[test]
    fn test_refactor_major_zero_patch_gt0() {
        // major==0 downgrades major bump to minor-level: patch > 0 → minor bump
        let v = parse_version("0.5.2-dev3").unwrap();
        assert_eq!(
            compute_bump(&v, CommitImpact::Refactor).to_string(),
            "0.6.0-dev0"
        );
    }

    #[test]
    fn test_refactor_major_zero_patch_zero() {
        // major==0 downgrades major bump to minor-level: patch == 0 → dev bump
        let v = parse_version("0.5.0-dev3").unwrap();
        assert_eq!(
            compute_bump(&v, CommitImpact::Refactor).to_string(),
            "0.5.0-dev4"
        );
    }

    #[test]
    fn test_refactor_at_x00() {
        let v = parse_version("1.0.0-dev5").unwrap();
        assert_eq!(
            compute_bump(&v, CommitImpact::Refactor).to_string(),
            "1.0.0-dev6"
        );
    }

    #[test]
    fn test_parse_version_with_dev() {
        let v = parse_version("1.3.0-dev10").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 3);
        assert_eq!(v.patch, 0);
        assert_eq!(v.dev, Some(10));
    }

    #[test]
    fn test_parse_version_stable() {
        let v = parse_version("1.3.0").unwrap();
        assert_eq!(v.dev, None);
    }

    #[test]
    fn test_next_dev_patch() {
        let v = Version { major: 1, minor: 3, patch: 0, dev: None };
        assert_eq!(
            next_dev_version(&v, NextDevTarget::Patch).to_string(),
            "1.3.1-dev0"
        );
    }

    #[test]
    fn test_next_dev_minor() {
        let v = Version { major: 1, minor: 3, patch: 0, dev: None };
        assert_eq!(
            next_dev_version(&v, NextDevTarget::Minor).to_string(),
            "1.4.0-dev0"
        );
    }

    #[test]
    fn test_strip_dev_suffix() {
        let v = parse_version("1.3.0-dev5").unwrap();
        assert_eq!(strip_dev_suffix(&v).to_string(), "1.3.0");
    }

    #[test]
    fn test_is_stable() {
        assert!(!is_stable(&parse_version("1.3.0-dev5").unwrap()));
        assert!(is_stable(&parse_version("1.3.0").unwrap()));
    }
}
