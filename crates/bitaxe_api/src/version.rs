use std::fmt;
use std::str::FromStr;

/// An ESP-Miner firmware version, such as `v2.15.4`.
///
/// Only the numeric `major.minor.patch` prefix is compared. Pre-release and build suffixes
/// (`v2.13.0b1`, `v2.15.0-34-ge3cfbb3`) are ignored, so a pre-release is treated as the release
/// it leads up to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FirmwareVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl FirmwareVersion {
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Parses the version reported by the device or used as a GitHub release tag.
    ///
    /// Returns `None` for strings that do not start with a numeric version, such as custom
    /// builds identified only by a commit hash.
    pub fn parse(value: &str) -> Option<Self> {
        let value = value.trim();
        let value = value
            .strip_prefix('v')
            .or_else(|| value.strip_prefix('V'))
            .unwrap_or(value);

        let mut parts = value.splitn(3, '.');
        let major = leading_number(parts.next()?)?;
        let minor = leading_number(parts.next()?)?;
        let patch = parts.next().and_then(leading_number).unwrap_or(0);

        Some(Self::new(major, minor, patch))
    }
}

impl FromStr for FirmwareVersion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| format!("'{s}' is not a firmware version like v2.15.4"))
    }
}

impl fmt::Display for FirmwareVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}.{}.{}", self.major, self.minor, self.patch)
    }
}

fn leading_number(part: &str) -> Option<u32> {
    let end = part
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(part.len());

    part[..end].parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_release_versions() {
        assert_eq!(
            FirmwareVersion::parse("v2.15.4"),
            Some(FirmwareVersion::new(2, 15, 4))
        );
        assert_eq!(
            FirmwareVersion::parse("2.9.0"),
            Some(FirmwareVersion::new(2, 9, 0))
        );
    }

    #[test]
    fn ignores_pre_release_and_build_suffixes() {
        assert_eq!(
            FirmwareVersion::parse("v2.13.0b1"),
            Some(FirmwareVersion::new(2, 13, 0))
        );
        assert_eq!(
            FirmwareVersion::parse("v2.15.2rc0"),
            Some(FirmwareVersion::new(2, 15, 2))
        );
        assert_eq!(
            FirmwareVersion::parse("v2.15.0-34-ge3cfbb3-dirty"),
            Some(FirmwareVersion::new(2, 15, 0))
        );
        assert_eq!(
            FirmwareVersion::parse("v1.0"),
            Some(FirmwareVersion::new(1, 0, 0))
        );
    }

    #[test]
    fn rejects_non_versions() {
        assert_eq!(FirmwareVersion::parse("e3cfbb3"), None);
        assert_eq!(FirmwareVersion::parse(""), None);
        assert_eq!(FirmwareVersion::parse("early-access-2026-03"), None);
    }

    #[test]
    fn orders_numerically() {
        assert!(FirmwareVersion::new(2, 10, 0) > FirmwareVersion::new(2, 9, 0));
        assert!(FirmwareVersion::new(2, 15, 0) > FirmwareVersion::new(2, 14, 2));
    }
}
