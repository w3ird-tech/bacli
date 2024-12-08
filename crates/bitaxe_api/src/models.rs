use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use serde_with::{serde_as, skip_serializing_none, BoolFromInt};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Error connecting to the API: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Invalid request - status {0}")]
    InvalidRequest(reqwest::StatusCode),
    #[error("Server error on API call - status {0}, body '{1}'")]
    ApiServer(reqwest::StatusCode, String),
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    #[serde(rename = "ASICModel")]
    pub asic_model: String,
    pub asic_count: i64,
    #[serde_as(as = "BoolFromInt")]
    pub autofanspeed: bool,
    pub best_diff: String,
    pub best_session_diff: String,
    pub board_version: String,
    pub core_voltage: i64,
    pub core_voltage_actual: i64,
    pub current: f64,
    pub fallback_stratum_port: i64,
    #[serde(rename = "fallbackStratumURL")]
    pub fallback_stratum_url: String,
    pub fallback_stratum_user: String,
    #[serde(rename = "fanrpm")]
    pub fan_rpm: i64,
    #[serde(rename = "fanspeed")]
    pub fan_speed: i64,
    #[serde(rename = "flipscreen")]
    #[serde_as(as = "BoolFromInt")]
    pub flip_screen: bool,
    pub free_heap: i64,
    pub frequency: i64,
    pub hash_rate: f64,
    pub hostname: String,
    #[serde(rename = "invertfanpolarity")]
    #[serde_as(as = "BoolFromInt")]
    pub invert_fan_polarity: bool,
    #[serde(rename = "invertscreen")]
    #[serde_as(as = "BoolFromInt")]
    pub invert_screen: bool,
    pub is_using_fallback_stratum: i64,
    pub mac_addr: String,
    #[serde(rename = "overheat_mode")]
    #[serde_as(as = "BoolFromInt")]
    pub overheat_mode: bool,
    pub power: f64,
    pub running_partition: String,
    pub shares_accepted: i64,
    pub shares_rejected: i64,
    pub small_core_count: i64,
    pub ssid: String,
    pub stratum_port: i64,
    #[serde(rename = "stratumURL")]
    pub stratum_url: String,
    pub stratum_user: String,
    pub temp: f64,
    pub uptime_seconds: u64,
    pub version: String,
    pub voltage: f64,
    pub vr_temp: i64,
    pub wifi_status: String,
}

/// Configurable settings available to change
#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[serde(rename_all = "camelCase")]
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
    #[serde(rename = "stratumURL")]
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
    #[serde(rename = "fallbackStratumURL")]
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

    /// The percentage of the fan speed to be applied.
    #[cfg_attr(feature = "clap", arg(long))]
    pub fanspeed: Option<u8>,

    /// Enable/disable automatic fan speed management.
    #[serde_as(as = "Option<BoolFromInt>")]
    #[cfg_attr(feature = "clap", arg(long))]
    pub autofanspeed: Option<bool>,

    /// Core voltage
    #[cfg_attr(feature = "clap", arg(long))]
    pub core_voltage: Option<Voltage>,

    /// Frequency
    #[cfg_attr(feature = "clap", arg(long))]
    pub frequency: Option<Frequency>,

    /// Whether to flip the screen.
    #[cfg_attr(feature = "clap", arg(long))]
    #[serde(rename = "flipscreen")]
    #[serde_as(as = "Option<BoolFromInt>")]
    pub flip_screen: Option<bool>,

    /// Whether to invert the fan polarity.
    #[cfg_attr(feature = "clap", arg(long))]
    #[serde(rename = "invertfanpolarity")]
    #[serde_as(as = "Option<BoolFromInt>")]
    pub invert_fan_polarity: Option<bool>,

    /// Whether to invert the screen.
    #[cfg_attr(feature = "clap", arg(long))]
    #[serde(rename = "invertscreen")]
    #[serde_as(as = "Option<BoolFromInt>")]
    pub invert_screen: Option<bool>,

    /// Enable/disable overheat mode. This would usually be used to disable overheat mode after
    /// an it has been automatically enabled when an overheat is detected.
    #[cfg_attr(feature = "clap", arg(long))]
    #[serde(rename = "overheat_mode")]
    #[serde_as(as = "Option<BoolFromInt>")]
    pub overheat_mode: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(u16)]
pub enum Frequency {
    FourHundred = 400,
    FourHundredNinety = 490,
    #[default]
    FiveHundredTwentyFive = 525,
    FiveHundredFifty = 550,
    FiveHundredSeventyFive = 575,
    SixHundred = 600,
}

#[cfg(feature = "clap")]
impl clap::ValueEnum for Frequency {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            Self::FourHundred,
            Self::FourHundredNinety,
            Self::FiveHundredTwentyFive,
            Self::FiveHundredFifty,
            Self::FiveHundredSeventyFive,
            Self::SixHundred,
        ]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        let value = match self {
            Self::FourHundred => "400",
            Self::FourHundredNinety => "490",
            Self::FiveHundredTwentyFive => "525",
            Self::FiveHundredFifty => "550",
            Self::FiveHundredSeventyFive => "575",
            Self::SixHundred => "600",
        };

        Some(value.into())
    }
}

#[derive(Debug, Clone, Default, Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(u16)]
pub enum Voltage {
    OneThousand = 1000,
    OneThousandSixty = 1060,
    OneThousandOneHundred = 1100,
    #[default]
    OneThousandOneHundredFifty = 1150,
    OneThousandTwoHundred = 1200,
    OneThousandTwoHundredFifty = 1250,
}

#[cfg(feature = "clap")]
impl clap::ValueEnum for Voltage {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            Self::OneThousand,
            Self::OneThousandSixty,
            Self::OneThousandOneHundred,
            Self::OneThousandOneHundredFifty,
            Self::OneThousandTwoHundred,
            Self::OneThousandTwoHundredFifty,
        ]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        let value = match self {
            Self::OneThousand => "1000",
            Self::OneThousandSixty => "1060",
            Self::OneThousandOneHundred => "1100",
            Self::OneThousandOneHundredFifty => "1150",
            Self::OneThousandTwoHundred => "1200",
            Self::OneThousandTwoHundredFifty => "1250",
        };

        Some(value.into())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn ensure_bools_parse_correctly() {
        let input = serde_json::json!({
            "autofanspeed": 1,
            "flipscreen": 0,
            "invertfanpolarity": 1,
            "invertscreen": 1,
            "overheat_mode": 0,
        });
        let settings: Settings = serde_json::from_value(input).unwrap();

        assert!(settings.autofanspeed.unwrap());
        assert!(!settings.flip_screen.unwrap());
        assert!(settings.invert_fan_polarity.unwrap());
        assert!(settings.invert_screen.unwrap());
        assert!(!settings.overheat_mode.unwrap());
    }

    #[test]
    fn ensure_bools_output_correctly() {
        let settings = Settings {
            autofanspeed: Some(true),
            flip_screen: Some(false),
            invert_fan_polarity: Some(true),
            invert_screen: Some(true),
            overheat_mode: Some(false),
            ..Default::default()
        };
        let output = serde_json::to_value(settings).unwrap();

        assert_eq!(output.get("autofanspeed").unwrap().as_i64().unwrap(), 1);
        assert_eq!(output.get("flipscreen").unwrap().as_i64().unwrap(), 0);
        assert_eq!(
            output.get("invertfanpolarity").unwrap().as_i64().unwrap(),
            1
        );
        assert_eq!(output.get("invertscreen").unwrap().as_i64().unwrap(), 1);
        assert_eq!(output.get("overheat_mode").unwrap().as_i64().unwrap(), 0);
    }

    #[test]
    fn ensure_frequency_parses_correctly() {
        let input = serde_json::json!({
            "frequency": 400
        });
        let settings: Settings = serde_json::from_value(input).unwrap();

        assert_eq!(settings.frequency.unwrap(), Frequency::FourHundred);
    }

    #[test]
    fn ensure_frequency_outputs_correctly() {
        let settings = Settings {
            frequency: Some(Frequency::SixHundred),
            ..Default::default()
        };
        let output = serde_json::to_value(settings).unwrap();

        assert_eq!(output.get("frequency").unwrap().as_i64().unwrap(), 600);
    }

    #[test]
    fn ensure_voltage_parses_correctly() {
        let input = serde_json::json!({
            "coreVoltage": 1250
        });
        let settings: Settings = serde_json::from_value(input).unwrap();

        assert_eq!(
            settings.core_voltage.unwrap(),
            Voltage::OneThousandTwoHundredFifty
        );
    }

    #[test]
    fn ensure_voltage_outputs_correctly() {
        let settings = Settings {
            core_voltage: Some(Voltage::OneThousandOneHundred),
            ..Default::default()
        };
        let output = serde_json::to_value(settings).unwrap();

        assert_eq!(output.get("coreVoltage").unwrap().as_i64().unwrap(), 1100);
    }
}
