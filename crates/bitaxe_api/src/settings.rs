//! Translates [`Settings`] into the requests a specific firmware version understands.
//!
//! The version boundaries come from the ESP-Miner `PATCH /api/system` handler and
//! `/api/system/pools/{id}` endpoint in each release.

use serde_json::{Map, Value};

use crate::models::{Error, Result, Rotation, Settings, SystemInfo};
use crate::version::FirmwareVersion;

/// Partial settings updates. Earlier releases read every field unconditionally.
const PARTIAL_UPDATES: FirmwareVersion = FirmwareVersion::new(2, 0, 4);
/// `fanspeed`, `autofanspeed`, and `invertfanpolarity`.
const FAN_SETTINGS: FirmwareVersion = FirmwareVersion::new(2, 0, 2);
const STRATUM_PASSWORD: FirmwareVersion = FirmwareVersion::new(2, 0, 7);
const HOSTNAME: FirmwareVersion = FirmwareVersion::new(2, 1, 5);
const OVERHEAT_MODE: FirmwareVersion = FirmwareVersion::new(2, 1, 10);
const FALLBACK_POOL: FirmwareVersion = FirmwareVersion::new(2, 3, 0);
/// `invertfanpolarity` was removed.
const NO_FAN_POLARITY: FirmwareVersion = FirmwareVersion::new(2, 7, 0);
/// `flipscreen` was replaced by `rotation`.
const ROTATION: FirmwareVersion = FirmwareVersion::new(2, 9, 0);
/// `fanspeed` was renamed to `manualFanSpeed` and the frequency became a float.
const MANUAL_FAN_SPEED: FirmwareVersion = FirmwareVersion::new(2, 11, 0);
/// Pools moved from top-level `stratum*` settings to `/api/system/pools/{id}`.
const POOL_SLOTS: FirmwareVersion = FirmwareVersion::new(2, 15, 0);

/// The requests needed to apply a set of settings.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct SettingsPlan {
    /// The body for `PATCH /api/system`, if any top-level settings changed.
    pub patch: Map<String, Value>,
    /// Pool slots to replace with `PUT /api/system/pools/{id}`.
    pub pools: Vec<(u16, Map<String, Value>)>,
}

struct Firmware<'a> {
    reported: &'a str,
    version: Option<FirmwareVersion>,
}

impl Firmware<'_> {
    /// Unknown versions are assumed to be recent builds.
    fn at_least(&self, version: FirmwareVersion) -> bool {
        self.version.is_none_or(|current| current >= version)
    }

    fn require(&self, setting: &'static str, since: FirmwareVersion) -> Result<()> {
        if self.at_least(since) {
            Ok(())
        } else {
            Err(self.unsupported(setting, format!("requires {since} or newer")))
        }
    }

    fn unsupported(&self, setting: &'static str, reason: String) -> Error {
        Error::UnsupportedSetting {
            setting,
            version: self.reported.to_string(),
            reason,
        }
    }
}

struct PoolChange<'a> {
    label: &'static str,
    url: Option<&'a str>,
    port: Option<u16>,
    user: Option<&'a str>,
    password: Option<&'a str>,
}

impl PoolChange<'_> {
    fn is_empty(&self) -> bool {
        self.url.is_none() && self.port.is_none() && self.user.is_none() && self.password.is_none()
    }
}

pub(crate) fn plan(settings: &Settings, info: &SystemInfo) -> Result<SettingsPlan> {
    let fw = Firmware {
        reported: &info.version,
        version: info.firmware_version(),
    };
    let mut plan = SettingsPlan::default();
    let patch = &mut plan.patch;

    if !fw.at_least(PARTIAL_UPDATES) {
        return Err(fw.unsupported(
            "update-settings",
            format!("partial settings updates require {PARTIAL_UPDATES} or newer"),
        ));
    }

    if let Some(hostname) = &settings.hostname {
        fw.require("hostname", HOSTNAME)?;
        patch.insert("hostname".into(), hostname.as_str().into());
    }
    if let Some(ssid) = &settings.ssid {
        patch.insert("ssid".into(), ssid.as_str().into());
    }
    if let Some(pass) = &settings.wifi_pass {
        patch.insert("wifiPass".into(), pass.as_str().into());
    }

    let primary = PoolChange {
        label: "main",
        url: settings.stratum_url.as_deref(),
        port: settings.stratum_port,
        user: settings.stratum_user.as_deref(),
        password: settings.stratum_password.as_deref(),
    };
    let fallback = PoolChange {
        label: "fallback",
        url: settings.fallback_stratum_url.as_deref(),
        port: settings.fallback_stratum_port,
        user: settings.fallback_stratum_user.as_deref(),
        password: settings.fallback_stratum_password.as_deref(),
    };

    if primary.password.is_some() {
        fw.require("stratum-password", STRATUM_PASSWORD)?;
    }
    if !fallback.is_empty() {
        fw.require("fallback-stratum-*", FALLBACK_POOL)?;
    }

    if fw.at_least(POOL_SLOTS) {
        let primary_index = info.primary_pool_index.unwrap_or(0);
        let fallback_index = info.secondary_pool_index.unwrap_or(1);

        if !fallback.is_empty() && fallback_index == primary_index {
            return Err(Error::InvalidSettings(format!(
                "the main and fallback pools both use slot {primary_index}; select a different \
                 fallback pool on the device first"
            )));
        }

        for (index, change) in [(primary_index, &primary), (fallback_index, &fallback)] {
            if !change.is_empty() {
                plan.pools.push((index, pool_body(info, index, change)?));
            }
        }
    } else {
        insert_legacy_pool(patch, "stratum", &primary);
        insert_legacy_pool(patch, "fallbackStratum", &fallback);
    }

    if let Some(speed) = settings.fanspeed {
        fw.require("fanspeed", FAN_SETTINGS)?;
        let key = if fw.at_least(MANUAL_FAN_SPEED) {
            "manualFanSpeed"
        } else {
            "fanspeed"
        };
        patch.insert(key.into(), speed.into());
    }
    if let Some(auto) = settings.autofanspeed {
        fw.require("autofanspeed", FAN_SETTINGS)?;
        patch.insert("autofanspeed".into(), flag(auto));
    }
    if let Some(voltage) = settings.core_voltage {
        patch.insert("coreVoltage".into(), voltage.into());
    }
    if let Some(frequency) = settings.frequency {
        patch.insert("frequency".into(), frequency_value(&fw, frequency)?);
    }

    match (settings.flip_screen, settings.rotation) {
        (Some(_), Some(_)) => {
            return Err(Error::InvalidSettings(
                "flip-screen and rotation cannot be set together".into(),
            ))
        }
        (Some(flip), None) if fw.at_least(ROTATION) => {
            let rotation = if flip {
                Rotation::OneHundredEighty
            } else {
                Rotation::Zero
            };
            patch.insert("rotation".into(), rotation.degrees().into());
        }
        (Some(flip), None) => {
            patch.insert("flipscreen".into(), flag(flip));
        }
        (None, Some(rotation)) if fw.at_least(ROTATION) => {
            patch.insert("rotation".into(), rotation.degrees().into());
        }
        (None, Some(rotation)) => {
            let flip = match rotation {
                Rotation::Zero => false,
                Rotation::OneHundredEighty => true,
                Rotation::Ninety | Rotation::TwoHundredSeventy => {
                    return Err(fw.unsupported(
                        "rotation",
                        format!("only 0 and 180 are supported before {ROTATION}"),
                    ))
                }
            };
            patch.insert("flipscreen".into(), flag(flip));
        }
        (None, None) => {}
    }

    if let Some(invert) = settings.invert_fan_polarity {
        fw.require("invert-fan-polarity", FAN_SETTINGS)?;
        if fw.at_least(NO_FAN_POLARITY) {
            return Err(fw.unsupported(
                "invert-fan-polarity",
                format!("the setting was removed in {NO_FAN_POLARITY}"),
            ));
        }
        patch.insert("invertfanpolarity".into(), flag(invert));
    }
    if let Some(invert) = settings.invert_screen {
        patch.insert("invertscreen".into(), flag(invert));
    }
    if let Some(overheat) = settings.overheat_mode {
        fw.require("overheat-mode", OVERHEAT_MODE)?;
        patch.insert("overheat_mode".into(), flag(overheat));
    }

    if plan.patch.is_empty() && plan.pools.is_empty() {
        return Err(Error::InvalidSettings("no settings were provided".into()));
    }

    Ok(plan)
}

fn insert_legacy_pool(patch: &mut Map<String, Value>, prefix: &str, change: &PoolChange) {
    if let Some(url) = change.url {
        patch.insert(format!("{prefix}URL"), url.into());
    }
    if let Some(port) = change.port {
        patch.insert(format!("{prefix}Port"), port.into());
    }
    if let Some(user) = change.user {
        patch.insert(format!("{prefix}User"), user.into());
    }
    if let Some(password) = change.password {
        patch.insert(format!("{prefix}Password"), password.into());
    }
}

/// Builds a full pool body for `PUT /api/system/pools/{id}`, which replaces the whole slot.
///
/// The current pool is used as the base so unrelated pool options are kept. The device reports
/// the password as `*****`, which it treats as "keep the stored password".
fn pool_body(info: &SystemInfo, index: u16, change: &PoolChange) -> Result<Map<String, Value>> {
    let mut body = match info.pools.iter().flatten().find(|p| p.id == index) {
        Some(pool) => match serde_json::to_value(pool) {
            Ok(Value::Object(map)) => map,
            _ => Map::new(),
        },
        None => Map::new(),
    };
    body.remove("id");

    if let Some(url) = change.url {
        body.insert("stratumURL".into(), url.into());
    }
    if let Some(port) = change.port {
        body.insert("stratumPort".into(), port.into());
    }
    if let Some(user) = change.user {
        body.insert("stratumUser".into(), user.into());
    }
    if let Some(password) = change.password {
        body.insert("stratumPassword".into(), password.into());
    }

    let has_text = |key: &str| {
        body.get(key)
            .and_then(Value::as_str)
            .is_some_and(|s| !s.is_empty())
    };
    if !has_text("stratumURL") || !body.contains_key("stratumPort") || !has_text("stratumUser") {
        let flag = if change.label == "main" {
            "stratum"
        } else {
            "fallback-stratum"
        };
        return Err(Error::InvalidSettings(format!(
            "the {} pool (slot {index}) is not configured on the device yet; provide \
             --{flag}-url, --{flag}-port, and --{flag}-user",
            change.label
        )));
    }

    Ok(body)
}

fn flag(value: bool) -> Value {
    u8::from(value).into()
}

fn frequency_value(fw: &Firmware, frequency: f64) -> Result<Value> {
    if !frequency.is_finite() || frequency <= 0.0 {
        return Err(Error::InvalidSettings(format!(
            "frequency must be a positive number, got {frequency}"
        )));
    }

    if frequency.fract() == 0.0 && frequency <= f64::from(u16::MAX) {
        return Ok((frequency as u16).into());
    }

    if !fw.at_least(MANUAL_FAN_SPEED) {
        return Err(fw.unsupported(
            "frequency",
            format!("fractional frequencies require {MANUAL_FAN_SPEED} or newer"),
        ));
    }

    serde_json::Number::from_f64(frequency)
        .map(Value::Number)
        .ok_or_else(|| Error::InvalidSettings(format!("invalid frequency {frequency}")))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn info(version: &str, extra: Value) -> SystemInfo {
        let mut base = json!({
            "version": version,
            "ASICModel": "BM1366",
            "hashRate": 500.0,
            "power": 12.5,
            "voltage": 5000.0,
            "current": 2500.0,
            "temp": 55.0,
            "bestDiff": "1.2M",
            "freeHeap": 100000,
            "coreVoltage": 1200,
            "coreVoltageActual": 1190,
            "frequency": 525,
            "ssid": "wifi",
            "wifiStatus": "Connected!",
            "sharesAccepted": 10,
            "sharesRejected": 0,
            "uptimeSeconds": 60,
            "stratumURL": "public-pool.io",
            "stratumPort": 21496,
            "stratumUser": "bc1q.worker",
            "runningPartition": "ota_0",
        });
        base.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        serde_json::from_value(base).unwrap()
    }

    fn patch_of(settings: Settings, version: &str) -> Value {
        Value::Object(plan(&settings, &info(version, json!({}))).unwrap().patch)
    }

    #[test]
    fn requires_at_least_one_setting() {
        let err = plan(&Settings::default(), &info("v2.15.4", json!({}))).unwrap_err();
        assert!(matches!(err, Error::InvalidSettings(_)));
    }

    #[test]
    fn rejects_firmware_without_partial_updates() {
        let settings = Settings {
            ssid: Some("wifi".into()),
            ..Default::default()
        };
        let err = plan(&settings, &info("v2.0.3", json!({}))).unwrap_err();
        assert!(matches!(err, Error::UnsupportedSetting { .. }));
    }

    #[test]
    fn fan_speed_key_follows_rename() {
        let settings = Settings {
            fanspeed: Some(80),
            autofanspeed: Some(false),
            ..Default::default()
        };

        assert_eq!(
            patch_of(settings.clone(), "v2.10.1"),
            json!({"fanspeed": 80, "autofanspeed": 0})
        );
        assert_eq!(
            patch_of(settings, "v2.11.0"),
            json!({"manualFanSpeed": 80, "autofanspeed": 0})
        );
    }

    #[test]
    fn flip_screen_maps_to_rotation_on_new_firmware() {
        let flip = Settings {
            flip_screen: Some(true),
            ..Default::default()
        };
        assert_eq!(patch_of(flip.clone(), "v2.8.1"), json!({"flipscreen": 1}));
        assert_eq!(patch_of(flip, "v2.9.0"), json!({"rotation": 180}));

        let upright = Settings {
            rotation: Some(Rotation::Zero),
            ..Default::default()
        };
        assert_eq!(patch_of(upright, "v2.8.1"), json!({"flipscreen": 0}));

        let sideways = Settings {
            rotation: Some(Rotation::Ninety),
            ..Default::default()
        };
        assert!(plan(&sideways, &info("v2.8.1", json!({}))).is_err());
        assert_eq!(patch_of(sideways, "v2.15.4"), json!({"rotation": 90}));
    }

    #[test]
    fn rejects_settings_outside_their_supported_range() {
        let polarity = Settings {
            invert_fan_polarity: Some(true),
            ..Default::default()
        };
        assert_eq!(
            patch_of(polarity.clone(), "v2.6.5"),
            json!({"invertfanpolarity": 1})
        );
        assert!(plan(&polarity, &info("v2.7.0", json!({}))).is_err());

        let hostname = Settings {
            hostname: Some("bitaxe".into()),
            ..Default::default()
        };
        assert!(plan(&hostname, &info("v2.1.4", json!({}))).is_err());
        assert_eq!(patch_of(hostname, "v2.1.5"), json!({"hostname": "bitaxe"}));
    }

    #[test]
    fn frequency_is_sent_as_integer_when_whole() {
        let whole = Settings {
            frequency: Some(525.0),
            core_voltage: Some(1150),
            ..Default::default()
        };
        assert_eq!(
            patch_of(whole, "v2.9.0"),
            json!({"frequency": 525, "coreVoltage": 1150})
        );

        let fractional = Settings {
            frequency: Some(512.5),
            ..Default::default()
        };
        assert!(plan(&fractional, &info("v2.10.1", json!({}))).is_err());
        assert_eq!(patch_of(fractional, "v2.11.0"), json!({"frequency": 512.5}));
    }

    #[test]
    fn legacy_pools_use_top_level_keys() {
        let settings = Settings {
            stratum_url: Some("pool.example".into()),
            stratum_password: Some("x".into()),
            fallback_stratum_port: Some(3333),
            ..Default::default()
        };

        assert_eq!(
            patch_of(settings.clone(), "v2.14.2"),
            json!({
                "stratumURL": "pool.example",
                "stratumPassword": "x",
                "fallbackStratumPort": 3333,
            })
        );
        assert!(plan(&settings, &info("v2.2.2", json!({}))).is_err());
    }

    #[test]
    fn pool_slots_are_updated_in_place() {
        let device = info(
            "v2.15.4",
            json!({
                "primaryPoolIndex": 2,
                "secondaryPoolIndex": 0,
                "pools": [
                    {
                        "id": 0,
                        "stratumURL": "fallback.example",
                        "stratumPort": 3333,
                        "stratumUser": "fb",
                        "stratumPassword": "*****",
                        "stratumTLS": 0,
                    },
                    {
                        "id": 2,
                        "stratumURL": "main.example",
                        "stratumPort": 4444,
                        "stratumUser": "main",
                        "stratumPassword": "*****",
                        "stratumProtocol": "v1",
                        "stratumTLS": 1,
                    },
                ],
            }),
        );
        let settings = Settings {
            stratum_user: Some("new-worker".into()),
            fallback_stratum_password: Some("secret".into()),
            ..Default::default()
        };

        let plan = plan(&settings, &device).unwrap();

        assert!(plan.patch.is_empty());
        assert_eq!(
            plan.pools,
            vec![
                (
                    2,
                    json!({
                        "stratumURL": "main.example",
                        "stratumPort": 4444,
                        "stratumUser": "new-worker",
                        "stratumPassword": "*****",
                        "stratumProtocol": "v1",
                        "stratumTLS": 1,
                    })
                    .as_object()
                    .unwrap()
                    .clone()
                ),
                (
                    0,
                    json!({
                        "stratumURL": "fallback.example",
                        "stratumPort": 3333,
                        "stratumUser": "fb",
                        "stratumPassword": "secret",
                        "stratumTLS": 0,
                    })
                    .as_object()
                    .unwrap()
                    .clone()
                ),
            ]
        );
    }

    #[test]
    fn empty_pool_slot_needs_full_configuration() {
        let device = info("v2.15.4", json!({"pools": []}));
        let partial = Settings {
            fallback_stratum_url: Some("fallback.example".into()),
            ..Default::default()
        };
        assert!(matches!(
            plan(&partial, &device),
            Err(Error::InvalidSettings(_))
        ));

        let full = Settings {
            fallback_stratum_url: Some("fallback.example".into()),
            fallback_stratum_port: Some(3333),
            fallback_stratum_user: Some("worker".into()),
            ..Default::default()
        };
        let plan = plan(&full, &device).unwrap();
        assert_eq!(plan.pools[0].0, 1);
        assert_eq!(
            Value::Object(plan.pools[0].1.clone()),
            json!({
                "stratumURL": "fallback.example",
                "stratumPort": 3333,
                "stratumUser": "worker",
            })
        );
    }

    #[test]
    fn unknown_versions_are_treated_as_current() {
        let settings = Settings {
            fanspeed: Some(50),
            ..Default::default()
        };
        assert_eq!(
            patch_of(settings, "custom-build"),
            json!({"manualFanSpeed": 50})
        );
    }
}
