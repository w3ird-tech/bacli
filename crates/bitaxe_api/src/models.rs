use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use serde_repr::{Deserialize_repr, Serialize_repr};
use serde_with::{serde_as, skip_serializing_none};

pub use crate::serde_utils::StringOrInt;
use crate::serde_utils::{LenientBool, LenientInt};
use crate::version::FirmwareVersion;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Error connecting to the API: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Invalid request - status {0}")]
    InvalidRequest(reqwest::StatusCode),
    #[error("Device rejected the request - status {0}, body '{1}'")]
    ApiClient(reqwest::StatusCode, String),
    #[error("Server error on API call - status {0}, body '{1}'")]
    ApiServer(reqwest::StatusCode, String),
    #[error("'{setting}' is not supported by firmware {version}: {reason}")]
    UnsupportedSetting {
        setting: &'static str,
        version: String,
        reason: String,
    },
    #[error("Invalid settings: {0}")]
    InvalidSettings(String),
}

/// The response of `GET /api/system/info`.
///
/// Fields that every ESP-Miner release since v2.0.0 reports are required. Everything else is
/// optional because it was added, renamed, or removed at some point. Fields this crate does not
/// model are kept in [`SystemInfo::other`].
#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    // Reported by every release since v2.0.0.
    pub version: String,
    #[serde(rename = "ASICModel")]
    pub asic_model: String,
    pub hash_rate: f64,
    pub power: f64,
    pub voltage: f64,
    pub current: f64,
    pub temp: f64,
    pub best_diff: StringOrInt,
    #[serde_as(as = "LenientInt")]
    pub free_heap: u64,
    #[serde_as(as = "LenientInt")]
    pub core_voltage: i64,
    pub core_voltage_actual: f64,
    pub frequency: f64,
    pub ssid: String,
    pub wifi_status: String,
    #[serde_as(as = "LenientInt")]
    pub shares_accepted: u64,
    #[serde_as(as = "LenientInt")]
    pub shares_rejected: u64,
    #[serde_as(as = "LenientInt")]
    pub uptime_seconds: u64,
    #[serde(rename = "stratumURL")]
    pub stratum_url: String,
    #[serde_as(as = "LenientInt")]
    pub stratum_port: u16,
    pub stratum_user: String,
    pub running_partition: String,

    // Added, renamed, or removed in some releases.
    #[serde(rename = "axeOSVersion")]
    pub axe_os_version: Option<String>,
    pub board_version: Option<String>,
    pub hostname: Option<String>,
    pub mac_addr: Option<String>,
    #[serde_as(as = "Option<LenientBool>")]
    pub ap_enabled: Option<bool>,
    #[serde_as(as = "Option<LenientBool>")]
    pub autofanspeed: Option<bool>,
    pub best_session_diff: Option<StringOrInt>,
    #[serde_as(as = "Option<LenientInt>")]
    pub display_timeout: Option<i64>,
    #[serde(rename = "fanrpm")]
    #[serde_as(as = "Option<LenientInt>")]
    pub fan_rpm: Option<u64>,
    /// Fan speed in percent.
    #[serde(rename = "fanspeed")]
    pub fan_speed: Option<f64>,
    #[serde_as(as = "Option<LenientInt>")]
    pub manual_fan_speed: Option<u64>,
    #[serde(rename = "invertscreen")]
    #[serde_as(as = "Option<LenientBool>")]
    pub invert_screen: Option<bool>,
    #[serde(rename = "isPSRAMAvailable")]
    #[serde_as(as = "Option<LenientBool>")]
    pub is_psram_available: Option<bool>,
    #[serde_as(as = "Option<LenientBool>")]
    pub is_using_fallback_stratum: Option<bool>,
    #[serde_as(as = "Option<LenientInt>")]
    pub max_power: Option<i64>,
    #[serde_as(as = "Option<LenientInt>")]
    pub nominal_voltage: Option<i64>,
    #[serde_as(as = "Option<LenientBool>")]
    pub overclock_enabled: Option<bool>,
    #[serde(rename = "overheat_mode")]
    #[serde_as(as = "Option<LenientBool>")]
    pub overheat_mode: Option<bool>,
    pub pool_difficulty: Option<f64>,
    pub rotation: Option<Rotation>,
    pub shares_rejected_reasons: Option<Vec<ShareRejectedReason>>,
    #[serde_as(as = "Option<LenientInt>")]
    pub small_core_count: Option<u64>,
    #[serde(rename = "temptarget")]
    pub temp_target: Option<f64>,
    pub vr_temp: Option<f64>,
    #[serde(rename = "wifiRSSI")]
    #[serde_as(as = "Option<LenientInt>")]
    pub wifi_rssi: Option<i64>,

    // Fallback pool, reported from v2.3.0.
    #[serde(rename = "fallbackStratumURL")]
    pub fallback_stratum_url: Option<String>,
    #[serde_as(as = "Option<LenientInt>")]
    pub fallback_stratum_port: Option<u16>,
    pub fallback_stratum_user: Option<String>,

    // Pool slots, reported from v2.15.0.
    pub pools: Option<Vec<Pool>>,
    #[serde_as(as = "Option<LenientInt>")]
    pub primary_pool_index: Option<u16>,
    #[serde_as(as = "Option<LenientInt>")]
    pub secondary_pool_index: Option<u16>,

    /// Any fields reported by the device that are not modeled above.
    #[serde(flatten)]
    pub other: Map<String, Value>,
}

impl SystemInfo {
    /// The parsed firmware version, or `None` for custom builds without a version number.
    pub fn firmware_version(&self) -> Option<FirmwareVersion> {
        FirmwareVersion::parse(&self.version)
    }
}

/// A configured pool slot, as reported by firmware v2.15.0 and newer.
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pool {
    #[serde_as(as = "LenientInt")]
    pub id: u16,
    #[serde(rename = "stratumURL")]
    pub stratum_url: String,
    #[serde_as(as = "LenientInt")]
    pub stratum_port: u16,
    pub stratum_user: String,
    /// The remaining pool fields, kept so a pool can be updated without losing them.
    #[serde(flatten)]
    pub other: Map<String, Value>,
}

#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(u16)]
pub enum Rotation {
    Zero = 0,
    Ninety = 90,
    OneHundredEighty = 180,
    TwoHundredSeventy = 270,
}

impl Rotation {
    pub fn degrees(self) -> u16 {
        self as u16
    }
}

#[cfg(feature = "clap")]
impl clap::ValueEnum for Rotation {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            Self::Zero,
            Self::Ninety,
            Self::OneHundredEighty,
            Self::TwoHundredSeventy,
        ]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        let value = match self {
            Self::Zero => "0",
            Self::Ninety => "90",
            Self::OneHundredEighty => "180",
            Self::TwoHundredSeventy => "270",
        };

        Some(value.into())
    }
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareRejectedReason {
    #[serde_as(as = "LenientInt")]
    pub count: u64,
    pub message: String,
}

/// Settings to change on the device.
///
/// These are firmware-independent. [`crate::client::BitaxeClient::update_settings`] translates
/// them into the request format the connected device's firmware understands, and returns
/// [`Error::UnsupportedSetting`] for settings the firmware cannot apply.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
pub struct Settings {
    /// The hostname of the device.
    #[cfg_attr(feature = "clap", arg(long))]
    pub hostname: Option<String>,

    /// The SSID of the Wifi network the device should connect to.
    #[cfg_attr(feature = "clap", arg(long))]
    pub ssid: Option<String>,

    /// The password of the Wifi network the device should connect to.
    #[cfg_attr(feature = "clap", arg(long))]
    pub wifi_pass: Option<String>,

    /// The URL of the main Stratum server to connect to.
    #[cfg_attr(feature = "clap", arg(long))]
    pub stratum_url: Option<String>,

    /// The port of the main Stratum server to connect to.
    #[cfg_attr(feature = "clap", arg(long))]
    pub stratum_port: Option<u16>,

    /// The username to use for the main Stratum server to connect to.
    #[cfg_attr(feature = "clap", arg(long))]
    pub stratum_user: Option<String>,

    /// The password to use for the main Stratum server to connect to.
    #[cfg_attr(feature = "clap", arg(long))]
    pub stratum_password: Option<String>,

    /// The URL of the fallback Stratum server to connect to.
    #[cfg_attr(feature = "clap", arg(long))]
    pub fallback_stratum_url: Option<String>,

    /// The port of the fallback Stratum server to connect to.
    #[cfg_attr(feature = "clap", arg(long))]
    pub fallback_stratum_port: Option<u16>,

    /// The username to use for the fallback Stratum server to connect to.
    #[cfg_attr(feature = "clap", arg(long))]
    pub fallback_stratum_user: Option<String>,

    /// The password to use for the fallback Stratum server to connect to.
    #[cfg_attr(feature = "clap", arg(long))]
    pub fallback_stratum_password: Option<String>,

    /// The fan speed, in percent, used when automatic fan speed is disabled.
    #[cfg_attr(feature = "clap", arg(long, value_parser = clap::value_parser!(u8).range(0..=100)))]
    pub fanspeed: Option<u8>,

    /// Enable/disable automatic fan speed management.
    #[cfg_attr(feature = "clap", arg(long))]
    pub autofanspeed: Option<bool>,

    /// ASIC core voltage in millivolts, e.g. 1200. The device validates the value for its ASIC.
    #[cfg_attr(feature = "clap", arg(long))]
    pub core_voltage: Option<u16>,

    /// ASIC frequency in MHz, e.g. 525. Fractional values require firmware v2.11.0 or newer.
    #[cfg_attr(feature = "clap", arg(long))]
    pub frequency: Option<f64>,

    /// Whether to flip the screen. On firmware v2.9.0 and newer this sets the rotation to 180
    /// (flipped) or 0 degrees.
    #[cfg_attr(feature = "clap", arg(long, conflicts_with = "rotation"))]
    pub flip_screen: Option<bool>,

    /// Screen rotation in degrees. Firmware older than v2.9.0 only supports 0 and 180.
    #[cfg_attr(feature = "clap", arg(long))]
    pub rotation: Option<Rotation>,

    /// Whether to invert the fan polarity. Only supported by firmware v2.0.2 through v2.6.x.
    #[cfg_attr(feature = "clap", arg(long))]
    pub invert_fan_polarity: Option<bool>,

    /// Whether to invert the screen.
    #[cfg_attr(feature = "clap", arg(long))]
    pub invert_screen: Option<bool>,

    /// Enable/disable overheat mode. This would usually be used to disable overheat mode after
    /// it has been automatically enabled when an overheat is detected.
    #[cfg_attr(feature = "clap", arg(long))]
    pub overheat_mode: Option<bool>,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn rotation_parses_degrees() {
        let rotation: Rotation = serde_json::from_value(serde_json::json!(180)).unwrap();
        assert_eq!(rotation, Rotation::OneHundredEighty);
        assert_eq!(rotation.degrees(), 180);
    }
}
