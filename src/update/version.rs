use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// SeeKi version in the format `YY.major.minor.patch[suffix]`.
///
/// Suffix (e.g. `a`, `b`, `rc1`) denotes a **pre-release**: versions
/// *without* a suffix are considered newer than the same numeric version
/// *with* a suffix.  For example `26.5.0.3` > `26.5.0.3a`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeekiVersion {
    pub year: u8,
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
    pub suffix: String,
}

impl SeekiVersion {
    /// Returns the version baked in at compile time.
    pub fn current() -> Self {
        env!("SEEKI_VERSION")
            .parse()
            .expect("SEEKI_VERSION set by build.rs must be a valid version")
    }

    /// `true` when a suffix is present (pre-release build).
    #[allow(dead_code)]
    pub fn is_pre_release(&self) -> bool {
        !self.suffix.is_empty()
    }
}

// ── Display ──────────────────────────────────────────────────────────────────

impl fmt::Display for SeekiVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.{}.{}.{}{}",
            self.year, self.major, self.minor, self.patch, self.suffix
        )
    }
}

// ── Parsing ──────────────────────────────────────────────────────────────────

impl FromStr for SeekiVersion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let parts: Vec<&str> = s.splitn(4, '.').collect();
        if parts.len() != 4 {
            return Err(format!(
                "expected format YY.major.minor.patch[suffix], got '{s}'"
            ));
        }

        let year: u8 = parts[0]
            .parse()
            .map_err(|_| format!("invalid year component: '{}'", parts[0]))?;
        let major: u16 = parts[1]
            .parse()
            .map_err(|_| format!("invalid major component: '{}'", parts[1]))?;
        let minor: u16 = parts[2]
            .parse()
            .map_err(|_| format!("invalid minor component: '{}'", parts[2]))?;

        // The fourth component may carry an alphabetic suffix, e.g. "3a"
        let patch_str = parts[3];
        let numeric_end = patch_str
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(patch_str.len());
        if numeric_end == 0 {
            return Err(format!(
                "patch component must start with a digit: '{patch_str}'"
            ));
        }
        let patch: u16 = patch_str[..numeric_end]
            .parse()
            .map_err(|_| format!("invalid patch component: '{}'", &patch_str[..numeric_end]))?;
        let suffix = patch_str[numeric_end..].to_string();

        // Validate suffix: must start with a letter, rest can be alphanumeric
        if !suffix.is_empty() {
            let mut chars = suffix.chars();
            let first = chars.next().unwrap();
            if !first.is_ascii_alphabetic() || !chars.all(|c| c.is_ascii_alphanumeric()) {
                return Err(format!(
                    "suffix must start with a letter and contain only letters/digits, got '{suffix}'"
                ));
            }
        }

        Ok(Self {
            year,
            major,
            minor,
            patch,
            suffix,
        })
    }
}

// ── Ordering ─────────────────────────────────────────────────────────────────
//
// Numeric fields are compared first.  When all numeric fields are equal,
// a version **without** a suffix is considered newer (greater) than one
// **with** a suffix.  Among two versions that both carry suffixes, the
// suffix is compared lexicographically.

impl PartialEq for SeekiVersion {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for SeekiVersion {}

impl PartialOrd for SeekiVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SeekiVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        self.year
            .cmp(&other.year)
            .then(self.major.cmp(&other.major))
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
            .then_with(|| {
                // No suffix  → stable (treated as *newer*)
                // Has suffix → pre-release
                match (self.suffix.is_empty(), other.suffix.is_empty()) {
                    (true, true) => Ordering::Equal,
                    (true, false) => Ordering::Greater, // stable > pre-release
                    (false, true) => Ordering::Less,    // pre-release < stable
                    (false, false) => self.suffix.cmp(&other.suffix),
                }
            })
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_version() {
        let v: SeekiVersion = "26.5.0.3".parse().unwrap();
        assert_eq!(v.year, 26);
        assert_eq!(v.major, 5);
        assert_eq!(v.minor, 0);
        assert_eq!(v.patch, 3);
        assert!(v.suffix.is_empty());
        assert!(!v.is_pre_release());
    }

    #[test]
    fn parse_version_with_suffix() {
        let v: SeekiVersion = "26.5.0.3a".parse().unwrap();
        assert_eq!(v.year, 26);
        assert_eq!(v.major, 5);
        assert_eq!(v.minor, 0);
        assert_eq!(v.patch, 3);
        assert_eq!(v.suffix, "a");
        assert!(v.is_pre_release());
    }

    #[test]
    fn parse_version_with_multi_char_suffix() {
        let v: SeekiVersion = "26.5.0.3rc".parse().unwrap();
        assert_eq!(v.patch, 3);
        assert_eq!(v.suffix, "rc");
        assert!(v.is_pre_release());
    }

    #[test]
    fn parse_trims_whitespace() {
        let v: SeekiVersion = "  26.5.0.3a  ".parse().unwrap();
        assert_eq!(v.year, 26);
        assert_eq!(v.suffix, "a");
    }

    #[test]
    fn display_roundtrip() {
        let input = "26.5.0.3a";
        let v: SeekiVersion = input.parse().unwrap();
        assert_eq!(v.to_string(), input);
    }

    #[test]
    fn display_no_suffix() {
        let v: SeekiVersion = "26.5.0.3".parse().unwrap();
        assert_eq!(v.to_string(), "26.5.0.3");
    }

    #[test]
    fn reject_too_few_parts() {
        assert!("26.5.0".parse::<SeekiVersion>().is_err());
    }

    #[test]
    fn reject_non_numeric_year() {
        assert!("abc.5.0.3".parse::<SeekiVersion>().is_err());
    }

    #[test]
    fn reject_empty_patch() {
        assert!("26.5.0.".parse::<SeekiVersion>().is_err());
    }

    #[test]
    fn accept_numeric_suffix() {
        let v: SeekiVersion = "26.5.0.3a1".parse().unwrap();
        assert_eq!(v.patch, 3);
        assert_eq!(v.suffix, "a1");
        assert!(v.is_pre_release());
    }

    #[test]
    fn accept_rc1_suffix() {
        let v: SeekiVersion = "26.5.0.3rc1".parse().unwrap();
        assert_eq!(v.patch, 3);
        assert_eq!(v.suffix, "rc1");
        assert!(v.is_pre_release());
    }

    #[test]
    fn reject_digit_only_suffix() {
        assert!("26.5.0.31".parse::<SeekiVersion>().is_err()
            || "26.5.0.31".parse::<SeekiVersion>().unwrap().suffix.is_empty());
    }

    // ── Ordering tests ───────────────────────────────────────────────────────

    #[test]
    fn stable_newer_than_pre_release() {
        let stable: SeekiVersion = "26.5.0.3".parse().unwrap();
        let pre: SeekiVersion = "26.5.0.3a".parse().unwrap();
        assert!(stable > pre);
    }

    #[test]
    fn higher_patch_is_newer() {
        let a: SeekiVersion = "26.5.0.4".parse().unwrap();
        let b: SeekiVersion = "26.5.0.3".parse().unwrap();
        assert!(a > b);
    }

    #[test]
    fn higher_minor_is_newer() {
        let a: SeekiVersion = "26.5.1.0".parse().unwrap();
        let b: SeekiVersion = "26.5.0.99".parse().unwrap();
        assert!(a > b);
    }

    #[test]
    fn higher_major_is_newer() {
        let a: SeekiVersion = "26.6.0.0".parse().unwrap();
        let b: SeekiVersion = "26.5.99.99".parse().unwrap();
        assert!(a > b);
    }

    #[test]
    fn higher_year_is_newer() {
        let a: SeekiVersion = "27.0.0.0".parse().unwrap();
        let b: SeekiVersion = "26.99.99.99".parse().unwrap();
        assert!(a > b);
    }

    #[test]
    fn equal_versions_are_equal() {
        let a: SeekiVersion = "26.5.0.3a".parse().unwrap();
        let b: SeekiVersion = "26.5.0.3a".parse().unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn suffix_ordering_lexicographic() {
        let a: SeekiVersion = "26.5.0.3b".parse().unwrap();
        let b: SeekiVersion = "26.5.0.3a".parse().unwrap();
        assert!(a > b);
    }

    #[test]
    fn current_version_parses() {
        let v = SeekiVersion::current();
        assert_eq!(v.to_string(), env!("SEEKI_VERSION"));
    }
}
