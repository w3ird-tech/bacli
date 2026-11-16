//! `GET /api/system/info` responses across ESP-Miner releases.
//!
//! The fixtures use the exact keys and value types each release's source emits, with
//! representative values. `master-yyjson` reflects the unreleased firmware that serializes the
//! response with yyjson, which writes whole floats as `525.0`.

use bitaxe_api::models::{Rotation, StringOrInt, SystemInfo};
use bitaxe_api::version::FirmwareVersion;

fn parse(name: &str) -> SystemInfo {
    let path = format!("{}/tests/fixtures/{name}.json", env!("CARGO_MANIFEST_DIR"));
    let json = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    serde_json::from_str(&json).unwrap_or_else(|e| panic!("parse {name}: {e}"))
}

#[test]
fn parses_v2_0_0() {
    let info = parse("v2.0.0");

    assert_eq!(info.firmware_version(), Some(FirmwareVersion::new(2, 0, 0)));
    assert_eq!(info.best_diff, StringOrInt::String("4.29G".into()));
    assert_eq!(info.stratum_port, 21496);
    assert_eq!(info.board_version, None);
    assert_eq!(info.hostname, None);
    assert_eq!(info.fallback_stratum_url, None);
    assert_eq!(info.invert_screen, Some(true));
    assert!(info.other.contains_key("flipscreen"));
}

#[test]
fn parses_v2_9_0() {
    let info = parse("v2.9.0");

    assert_eq!(info.board_version.as_deref(), Some("601"));
    assert_eq!(info.rotation, Some(Rotation::OneHundredEighty));
    assert_eq!(info.pool_difficulty, Some(1000.0));
    assert_eq!(info.fan_speed, Some(42.0));
    assert_eq!(info.fallback_stratum_port, Some(3333));
    assert_eq!(info.is_using_fallback_stratum, Some(true));
    assert_eq!(info.display_timeout, Some(-1));
    assert_eq!(info.shares_rejected_reasons.unwrap()[0].count, 3);
}

#[test]
fn parses_v2_11_0_numeric_best_diff() {
    let info = parse("v2.11.0");

    assert_eq!(info.best_diff, StringOrInt::Int(4_294_967_296));
    assert_eq!(info.manual_fan_speed, Some(100));
    assert_eq!(info.pools.as_ref().map(Vec::len), None);
}

#[test]
fn parses_v2_15_4_pool_slots_and_fractional_frequency() {
    let info = parse("v2.15.4");

    assert_eq!(info.frequency, 512.5);
    assert_eq!(info.primary_pool_index, Some(0));
    assert_eq!(info.secondary_pool_index, Some(1));

    let pools = info.pools.unwrap();
    assert_eq!(pools.len(), 2);
    assert_eq!(pools[1].stratum_url, "solo.ckpool.org");
    assert_eq!(pools[1].other["stratumPassword"], "*****");
}

#[test]
fn parses_master_yyjson_floats() {
    let info = parse("master-yyjson");

    assert_eq!(
        info.firmware_version(),
        Some(FirmwareVersion::new(2, 15, 0))
    );
    assert_eq!(info.frequency, 525.0);
    assert_eq!(info.vr_temp, Some(47.0));
    assert_eq!(info.core_voltage_actual, 1146.0);
    assert_eq!(info.pool_difficulty, Some(1000.0));
}

#[test]
fn json_output_keeps_unmodeled_fields() {
    let info = parse("v2.15.4");
    let output = serde_json::to_value(&info).unwrap();

    assert_eq!(output["ASICModel"], "BM1370");
    assert_eq!(output["stratumURL"], "public-pool.io");
    assert_eq!(output["autofanspeed"], true);
    assert!(output.get("hashrateMonitor").is_some());
    assert!(output.get("idfVersion").is_some());
}

#[test]
fn json_output_omits_fields_the_device_did_not_report() {
    let info = parse("v2.0.0");
    let output = serde_json::to_value(&info).unwrap();

    assert!(output.get("hostname").is_none());
    assert!(output.get("pools").is_none());
}
