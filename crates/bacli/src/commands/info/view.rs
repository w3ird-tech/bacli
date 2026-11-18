//! Renders [`SystemInfo`] as the human-readable `bacli info` screen.
//!
//! Lines are built as styled spans first so that column widths can be measured on the visible
//! text, then painted with or without ANSI colors at the end.

use anstyle::{AnsiColor, Style};
use bitaxe_api::models::{StringOrInt, SystemInfo};
use serde_json::Value;

use super::dashboard::pool_dashboard;

const INDENT: &str = "  ";
const SEPARATOR: &str = " · ";
/// Spaces between a label and its value.
const LABEL_GAP: usize = 2;
/// Spaces between the left and right column of a row.
const COLUMN_GAP: usize = 4;

/// ASIC temperature, in °C, from which the reading is highlighted.
const ASIC_TEMP_WARN: f64 = 70.0;
const ASIC_TEMP_BAD: f64 = 75.0;
/// Voltage regulator temperature, in °C, from which the reading is highlighted.
const VR_TEMP_WARN: f64 = 85.0;
const VR_TEMP_BAD: f64 = 100.0;
/// Share reject rate, in percent, from which the rate is highlighted.
const REJECT_RATE_WARN: f64 = 1.0;
const REJECT_RATE_BAD: f64 = 5.0;
/// Wifi signal strength, in dBm, at or below which the reading is highlighted.
const RSSI_WARN: i64 = -75;
const RSSI_BAD: i64 = -85;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tone {
    Plain,
    Dim,
    Bold,
    Good,
    Warn,
    Bad,
}

impl Tone {
    fn style(self) -> Style {
        match self {
            Tone::Plain => Style::new(),
            Tone::Dim => Style::new().dimmed(),
            Tone::Bold => Style::new().bold(),
            Tone::Good => Style::new().fg_color(Some(AnsiColor::Green.into())),
            Tone::Warn => Style::new().fg_color(Some(AnsiColor::Yellow.into())),
            Tone::Bad => Style::new().bold().fg_color(Some(AnsiColor::Red.into())),
        }
    }

    /// Picks a tone for a reading where higher is worse.
    fn rising(value: f64, warn: f64, bad: f64) -> Self {
        if value >= bad {
            Tone::Bad
        } else if value >= warn {
            Tone::Warn
        } else {
            Tone::Plain
        }
    }
}

/// A run of styled text.
#[derive(Debug, Clone, Default)]
struct Text(Vec<(String, Tone)>);

impl Text {
    fn new(text: impl Into<String>, tone: Tone) -> Self {
        Self::default().push(text, tone)
    }

    fn plain(text: impl Into<String>) -> Self {
        Self::new(text, Tone::Plain)
    }

    /// Appends text directly after what is already there.
    fn push(mut self, text: impl Into<String>, tone: Tone) -> Self {
        self.0.push((text.into(), tone));
        self
    }

    /// Appends text as a new part, separated from any earlier parts.
    fn part(self, text: impl Into<String>, tone: Tone) -> Self {
        if self.0.is_empty() {
            self.push(text, tone)
        } else {
            self.push(SEPARATOR, Tone::Dim).push(text, tone)
        }
    }

    fn append(mut self, other: Text) -> Self {
        self.0.extend(other.0);
        self
    }

    fn width(&self) -> usize {
        self.0.iter().map(|(text, _)| text.chars().count()).sum()
    }

    fn pad_to(self, width: usize) -> Self {
        let missing = width.saturating_sub(self.width());
        if missing == 0 {
            self
        } else {
            self.push(" ".repeat(missing), Tone::Plain)
        }
    }

    fn paint(&self, color: bool) -> String {
        let mut out = String::new();
        for (text, tone) in &self.0 {
            if color && *tone != Tone::Plain {
                let style = tone.style();
                out.push_str(&format!("{style}{text}{style:#}"));
            } else {
                out.push_str(text);
            }
        }
        out
    }
}

struct Field {
    label: String,
    value: Text,
}

impl Field {
    fn new(label: impl Into<String>, value: Text) -> Self {
        Self {
            label: label.into(),
            value,
        }
    }
}

#[derive(Clone, Copy)]
enum Marker {
    None,
    Active,
    Inactive,
}

struct Row {
    marker: Marker,
    left: Field,
    right: Option<Field>,
    /// Whether the row may run past the rest of the screen without widening it. Used for values
    /// that cannot be shortened, such as links.
    overflow: bool,
}

impl Row {
    fn single(field: Field) -> Self {
        Self {
            marker: Marker::None,
            left: field,
            right: None,
            overflow: false,
        }
    }

    /// A row holding a value that is kept whole however long it is.
    fn overflowing(field: Field) -> Self {
        Self {
            overflow: true,
            ..Self::single(field)
        }
    }

    /// Lays out up to two fields side by side, skipping the row when the device reports neither.
    fn pair(left: Option<Field>, right: Option<Field>) -> Option<Self> {
        let (left, right) = match (left, right) {
            (Some(left), right) => (left, right),
            (None, Some(right)) => (right, None),
            (None, None) => return None,
        };

        Some(Self {
            marker: Marker::None,
            left,
            right,
            overflow: false,
        })
    }
}

struct Section {
    title: &'static str,
    rows: Vec<Row>,
}

/// Builds the info screen for the device reachable at `base`.
pub fn render(base: &str, info: &SystemInfo, color: bool) -> String {
    let sections: Vec<Section> = [
        ("Mining", mining(info)),
        ("Power & Thermals", power_and_thermals(info)),
        ("Network", network(info)),
        ("Pools", pools(info)),
    ]
    .into_iter()
    .filter(|(_, rows)| !rows.is_empty())
    .map(|(title, rows)| Section { title, rows })
    .collect();

    let (body, body_width) = layout(&sections);

    let mut lines = vec![title_line(base, info, body_width), subtitle_line(info)];
    if info.overheat_mode == Some(true) {
        lines.push(Text::default());
        lines.push(Text::new("! Overheat mode is active.", Tone::Bad));
        lines.push(Text::plain(format!(
            "  Fix the cooling, then clear it: bacli update-settings {base} --overheat-mode false"
        )));
    }
    lines.extend(body);

    let mut out = String::new();
    for line in lines {
        out.push_str(line.paint(color).trim_end());
        out.push('\n');
    }
    out
}

/// Turns sections into lines, sharing column widths across all of them. Also returns the width of
/// the widest line, leaving out rows that are allowed to overflow.
fn layout(sections: &[Section]) -> (Vec<Text>, usize) {
    let rows = || sections.iter().flat_map(|s| &s.rows);
    let width = |label: &str| label.chars().count();

    let label_width = rows().map(|r| width(&r.left.label)).max().unwrap_or(0);
    let left_width = rows()
        .filter(|r| r.right.is_some())
        .map(|r| r.left.value.width())
        .max()
        .unwrap_or(0);
    let right_label_width = rows()
        .filter_map(|r| r.right.as_ref())
        .map(|f| width(&f.label))
        .max()
        .unwrap_or(0);

    let left_start = INDENT.len() + label_width + LABEL_GAP;
    let right_start = left_start + left_width + COLUMN_GAP;

    let mut lines = Vec::new();
    let mut body_width = 0;
    for section in sections {
        lines.push(Text::default());
        lines.push(Text::new(section.title, Tone::Bold));

        for row in &section.rows {
            let marker = match row.marker {
                Marker::None => Text::plain(INDENT),
                Marker::Active => Text::new("● ", Tone::Good),
                Marker::Inactive => Text::new("○ ", Tone::Dim),
            };
            let mut line = marker
                .push(row.left.label.as_str(), Tone::Dim)
                .pad_to(left_start)
                .append(row.left.value.clone());

            if let Some(right) = &row.right {
                line = line
                    .pad_to(right_start)
                    .push(right.label.as_str(), Tone::Dim)
                    .pad_to(right_start + right_label_width + LABEL_GAP)
                    .append(right.value.clone());
            }

            if !row.overflow {
                body_width = body_width.max(line.width());
            }
            lines.push(line);
        }
    }
    (lines, body_width)
}

fn title_line(base: &str, info: &SystemInfo, body_width: usize) -> Text {
    let mut title = Text::default();
    if let Some(hostname) = present(info.hostname.as_deref()).filter(|h| *h != base) {
        title = title.part(hostname, Tone::Bold);
    }
    title = title.part(base, Tone::Bold);

    let uptime = Text::new(format!("up {}", uptime(info.uptime_seconds)), Tone::Dim);
    let uptime_start = body_width
        .saturating_sub(uptime.width())
        .max(title.width() + COLUMN_GAP);

    title.pad_to(uptime_start).append(uptime)
}

fn subtitle_line(info: &SystemInfo) -> Text {
    let mut subtitle = Text::default();
    if let Some(board) = present(info.board_version.as_deref()) {
        subtitle = subtitle.part(format!("Board {board}"), Tone::Plain);
    }
    subtitle = subtitle
        .part(info.asic_model.as_str(), Tone::Plain)
        .part(format!("ESP-Miner {}", info.version), Tone::Plain);

    match present(info.axe_os_version.as_deref()).filter(|v| *v != info.version) {
        Some(axe_os) => subtitle.push(format!(" (AxeOS {axe_os})"), Tone::Dim),
        None => subtitle,
    }
}

fn mining(info: &SystemInfo) -> Vec<Row> {
    let mut rows = Vec::new();

    let hashrate_tone = if info.hash_rate > 0.0 {
        Tone::Bold
    } else {
        Tone::Warn
    };
    rows.extend(Row::pair(
        Some(Field::new(
            "Hashrate",
            Text::new(hashrate(info.hash_rate), hashrate_tone),
        )),
        Some(Field::new(
            "Frequency",
            Text::plain(format!("{} MHz", number(info.frequency, 1))),
        )),
    ));

    let mut shares = Text::plain(format!("{} accepted", grouped(info.shares_accepted))).part(
        format!("{} rejected", grouped(info.shares_rejected)),
        Tone::Plain,
    );
    let total = info.shares_accepted + info.shares_rejected;
    if total > 0 {
        let rate = info.shares_rejected as f64 / total as f64 * 100.0;
        shares = shares.push(
            format!(" ({rate:.2}%)"),
            match Tone::rising(rate, REJECT_RATE_WARN, REJECT_RATE_BAD) {
                Tone::Plain => Tone::Dim,
                tone => tone,
            },
        );
    }
    rows.push(Row::single(Field::new("Shares", shares)));

    let reasons: Vec<String> = info
        .shares_rejected_reasons
        .iter()
        .flatten()
        .filter(|reason| reason.count > 0)
        .map(|reason| format!("{} ×{}", reason.message, grouped(reason.count)))
        .collect();
    if !reasons.is_empty() {
        let text = reasons.into_iter().fold(Text::default(), |text, reason| {
            text.part(reason, Tone::Plain)
        });
        rows.push(Row::single(Field::new("Rejects", text)));
    }

    let mut best = Text::plain(difficulty(&info.best_diff));
    if let Some(session) = &info.best_session_diff {
        best = best.push(format!(" (session {})", difficulty(session)), Tone::Dim);
    }
    rows.extend(Row::pair(
        Some(Field::new("Best diff", best)),
        info.pool_difficulty
            .map(|diff| Field::new("Pool diff", Text::plain(si(diff)))),
    ));

    rows
}

fn power_and_thermals(info: &SystemInfo) -> Vec<Row> {
    let mut rows = Vec::new();

    // The firmware reports millivolts and milliamps.
    let power = Text::plain(format!("{:.1} W", info.power))
        .part(format!("{:.2} V", info.voltage / 1000.0), Tone::Plain)
        .part(format!("{:.2} A", info.current / 1000.0), Tone::Plain);
    let efficiency = (info.hash_rate > 0.0 && info.power > 0.0).then(|| {
        let joules_per_terahash = info.power / (info.hash_rate / 1000.0);
        Field::new(
            "Efficiency",
            Text::plain(format!("{joules_per_terahash:.1} J/TH")),
        )
    });
    rows.extend(Row::pair(Some(Field::new("Power", power)), efficiency));

    let mut asic_temp = Text::new(
        format!("{} °C", number(info.temp, 1)),
        Tone::rising(info.temp, ASIC_TEMP_WARN, ASIC_TEMP_BAD),
    );
    if let Some(target) = info.temp_target {
        asic_temp = asic_temp.push(format!(" (target {})", number(target, 1)), Tone::Dim);
    }
    let vr_temp = info.vr_temp.filter(|temp| *temp > 0.0).map(|temp| {
        Field::new(
            "VR temp",
            Text::new(
                format!("{} °C", number(temp, 1)),
                Tone::rising(temp, VR_TEMP_WARN, VR_TEMP_BAD),
            ),
        )
    });
    rows.extend(Row::pair(Some(Field::new("ASIC temp", asic_temp)), vr_temp));

    let core = Field::new(
        "Core",
        Text::plain(format!("{} mV", number(info.core_voltage_actual, 0)))
            .push(format!(" (set {})", info.core_voltage), Tone::Dim),
    );
    rows.extend(Row::pair(fan(info), Some(core)));

    rows
}

fn fan(info: &SystemInfo) -> Option<Field> {
    // Firmware older than v2.1.0 spelled the field `fanSpeed`.
    let speed = info
        .fan_speed
        .or_else(|| info.other.get("fanSpeed").and_then(Value::as_f64));

    let mut text = Text::default();
    if let Some(speed) = speed {
        text = text.part(format!("{}%", number(speed, 0)), Tone::Plain);
    }
    if let Some(rpm) = info.fan_rpm {
        text = text.part(format!("{} RPM", grouped(rpm)), Tone::Plain);
    }
    if text.width() == 0 {
        return None;
    }

    if let Some(auto) = info.autofanspeed {
        let mode = if auto { "auto" } else { "manual" };
        text = text.push(format!(" ({mode})"), Tone::Dim);
    }

    Some(Field::new("Fan", text))
}

fn network(info: &SystemInfo) -> Vec<Row> {
    let mut rows = Vec::new();

    let mut wifi = Text::default();
    if !info.ssid.is_empty() {
        wifi = wifi.part(info.ssid.as_str(), Tone::Plain);
    }
    if let Some(rssi) = info.wifi_rssi {
        let tone = if rssi <= RSSI_BAD {
            Tone::Bad
        } else if rssi <= RSSI_WARN {
            Tone::Warn
        } else {
            Tone::Plain
        };
        wifi = wifi.part(format!("{rssi} dBm"), tone);
    }
    if !info.wifi_status.is_empty() {
        let tone = if info.wifi_status.starts_with("Connected") {
            Tone::Good
        } else {
            Tone::Warn
        };
        wifi = wifi.part(info.wifi_status.as_str(), tone);
    }
    if wifi.width() > 0 {
        rows.push(Row::single(Field::new("WiFi", wifi)));
    }

    if let Some(mac) = present(info.mac_addr.as_deref()) {
        rows.push(Row::single(Field::new("MAC", Text::plain(mac))));
    }

    rows
}

fn pools(info: &SystemInfo) -> Vec<Row> {
    let on_fallback = info.is_using_fallback_stratum.unwrap_or(false);
    let marker = |active| {
        if active {
            Marker::Active
        } else {
            Marker::Inactive
        }
    };
    let pool_row = |label: String, active: bool, url: &str, port: u16, user: &str| {
        let mut value = Text::plain(format!("{url}:{port}"));
        if !user.is_empty() {
            value = value.part(shorten_user(user), Tone::Plain);
        }
        let mut rows = vec![Row {
            marker: marker(active),
            left: Field::new(label, value),
            right: None,
            overflow: false,
        }];
        // Only the pool in use gets a link, to where its shares are showing up right now.
        if let Some(link) = active.then(|| pool_dashboard(url, user)).flatten() {
            rows.push(Row::overflowing(Field::new("Dashboard", Text::plain(link))));
        }
        rows
    };

    // Firmware v2.15.0 and newer reports pool slots.
    let primary_index = info.primary_pool_index.unwrap_or(0);
    let fallback_index = info.secondary_pool_index.unwrap_or(1);
    let slots: Vec<Row> = info
        .pools
        .iter()
        .flatten()
        .filter(|pool| !pool.stratum_url.is_empty())
        .flat_map(|pool| {
            let (label, active) = if pool.id == primary_index {
                ("Primary".to_string(), !on_fallback)
            } else if pool.id == fallback_index {
                ("Fallback".to_string(), on_fallback)
            } else {
                (format!("Pool {}", pool.id), false)
            };
            pool_row(
                label,
                active,
                &pool.stratum_url,
                pool.stratum_port,
                &pool.stratum_user,
            )
        })
        .collect();
    if !slots.is_empty() {
        return slots;
    }

    let mut rows = pool_row(
        "Primary".to_string(),
        !on_fallback,
        &info.stratum_url,
        info.stratum_port,
        &info.stratum_user,
    );

    // Firmware older than v2.3.0 has no fallback pool, so the row is left out entirely.
    if let Some(url) = &info.fallback_stratum_url {
        rows.extend(match (url.is_empty(), info.fallback_stratum_port) {
            (false, Some(port)) => pool_row(
                "Fallback".to_string(),
                on_fallback,
                url,
                port,
                info.fallback_stratum_user.as_deref().unwrap_or_default(),
            ),
            _ => vec![Row {
                marker: Marker::Inactive,
                left: Field::new("Fallback", Text::new("not configured", Tone::Dim)),
                right: None,
                overflow: false,
            }],
        });
    }

    rows
}

fn present(value: Option<&str>) -> Option<&str> {
    value.filter(|v| !v.is_empty())
}

/// Shortens the address part of a `address.worker` pool user so the row stays readable.
fn shorten_user(user: &str) -> String {
    const MAX_ADDRESS: usize = 16;
    const HEAD: usize = 8;
    const TAIL: usize = 6;

    let (address, worker) = match user.split_once('.') {
        Some((address, worker)) => (address, Some(worker)),
        None => (user, None),
    };

    let chars: Vec<char> = address.chars().collect();
    let address = if chars.len() > MAX_ADDRESS {
        let head: String = chars[..HEAD].iter().collect();
        let tail: String = chars[chars.len() - TAIL..].iter().collect();
        format!("{head}…{tail}")
    } else {
        address.to_string()
    };

    match worker {
        Some(worker) => format!("{address}.{worker}"),
        None => address,
    }
}

/// Formats a hash rate the firmware reports in GH/s.
fn hashrate(gigahashes: f64) -> String {
    if gigahashes <= 0.0 {
        "0 GH/s".to_string()
    } else if gigahashes >= 1000.0 {
        format!("{:.2} TH/s", gigahashes / 1000.0)
    } else if gigahashes >= 1.0 {
        format!("{gigahashes:.1} GH/s")
    } else {
        format!("{:.1} MH/s", gigahashes * 1000.0)
    }
}

/// Formats a difficulty, which older firmware pre-formats (`"4.29G"`) and newer firmware reports
/// as a number.
fn difficulty(value: &StringOrInt) -> String {
    match value {
        StringOrInt::String(text) => text.clone(),
        StringOrInt::Int(number) => si(*number as f64),
        StringOrInt::Float(number) => si(*number),
    }
}

/// Formats a number with the metric suffixes the firmware uses for difficulties.
fn si(value: f64) -> String {
    const UNITS: [(f64, &str); 5] = [(1e15, "P"), (1e12, "T"), (1e9, "G"), (1e6, "M"), (1e3, "k")];

    match UNITS.iter().find(|(scale, _)| value.abs() >= *scale) {
        Some((scale, suffix)) => format!("{}{suffix}", number(value / scale, 2)),
        None => number(value, 0),
    }
}

/// Formats a float with at most `decimals` decimals, dropping trailing zeros.
fn number(value: f64, decimals: usize) -> String {
    let text = format!("{value:.decimals$}");
    if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        text
    }
}

/// Formats an integer with thousands separators.
fn grouped(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// Formats an uptime using its largest units, dropping seconds once it passes a minute.
fn uptime(seconds: u64) -> String {
    let units = [
        (seconds / 86_400, "d"),
        (seconds % 86_400 / 3_600, "h"),
        (seconds % 3_600 / 60, "m"),
    ];
    let parts: Vec<String> = units
        .iter()
        .filter(|(amount, _)| *amount > 0)
        .map(|(amount, unit)| format!("{amount}{unit}"))
        .collect();

    if parts.is_empty() {
        format!("{seconds}s")
    } else {
        parts.join(" ")
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn fixture(json: &str) -> SystemInfo {
        serde_json::from_str(json).unwrap()
    }

    fn latest() -> SystemInfo {
        fixture(include_str!(
            "../../../../bitaxe_api/tests/fixtures/v2.15.4.json"
        ))
    }

    fn oldest() -> SystemInfo {
        fixture(include_str!(
            "../../../../bitaxe_api/tests/fixtures/v2.0.0.json"
        ))
    }

    #[test]
    fn renders_everything_current_firmware_reports() {
        let expected = "\
bitaxe · 192.168.1.42                                             up 1d
Board 601 · BM1370 · ESP-Miner v2.15.4

! Overheat mode is active.
  Fix the cooling, then clear it: bacli update-settings 192.168.1.42 --overheat-mode false

Mining
  Hashrate   1.10 TH/s                   Frequency   512.5 MHz
  Shares     1,234 accepted · 3 rejected (0.24%)
  Rejects    Above target ×3
  Best diff  4.29G (session 1.02M)       Pool diff   1k

Power & Thermals
  Power      15.3 W · 5.10 V · 3.01 A    Efficiency  13.9 J/TH
  ASIC temp  58.5 °C (target 60)         VR temp     47 °C
  Fan        42% · 3,800 RPM (auto)      Core        1146 mV (set 1150)

Network
  WiFi       home-wifi · -56 dBm · Connected!
  MAC        AA:BB:CC:DD:EE:FF

Pools
○ Primary    public-pool.io:21496 · bc1qexample.worker
● Fallback   solo.ckpool.org:3333 · bc1qexample.fallback
";

        assert_eq!(render("192.168.1.42", &latest(), false), expected);
    }

    #[test]
    fn omits_what_old_firmware_does_not_report() {
        let expected = "\
192.168.1.42                                                      up 1d
BM1370 · ESP-Miner v2.0.0

Mining
  Hashrate   1.10 TH/s                   Frequency   525 MHz
  Shares     1,234 accepted · 3 rejected (0.24%)
  Best diff  4.29G

Power & Thermals
  Power      15.3 W · 5.10 V · 3.01 A    Efficiency  13.9 J/TH
  ASIC temp  58.5 °C
  Fan        42%                         Core        1146 mV (set 1150)

Network
  WiFi       home-wifi · Connected!

Pools
● Primary    public-pool.io:21496 · bc1qexample.worker
";

        assert_eq!(render("192.168.1.42", &oldest(), false), expected);
    }

    #[test]
    fn marks_the_primary_pool_active_and_an_empty_fallback_unconfigured() {
        let mut info = latest();
        info.pools = None;
        info.is_using_fallback_stratum = Some(false);
        info.fallback_stratum_url = Some(String::new());

        let output = render("192.168.1.42", &info, false);

        assert!(output.contains("● Primary    public-pool.io:21496"));
        assert!(output.contains("○ Fallback   not configured"));
    }

    #[test]
    fn links_the_dashboard_of_the_pool_in_use_without_widening_the_screen() {
        const ADDRESS: &str = "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq";

        let mut info = latest();
        for pool in info.pools.iter_mut().flatten() {
            pool.stratum_user = format!("{ADDRESS}.bitaxe");
        }
        let plain = render("192.168.1.42", &latest(), false);
        let output = render("192.168.1.42", &info, false);

        // The device is on its fallback pool, so the primary pool gets no link.
        assert!(output.ends_with(
            "\
Pools
○ Primary    public-pool.io:21496 · bc1qar0s…wf5mdq.bitaxe
● Fallback   solo.ckpool.org:3333 · bc1qar0s…wf5mdq.bitaxe
  Dashboard  https://stats.ckpool.org/users/bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq
"
        ));
        assert_eq!(output.lines().next(), plain.lines().next());

        info.is_using_fallback_stratum = Some(false);
        let output = render("192.168.1.42", &info, false);

        assert!(output.ends_with(
            "\
Pools
● Primary    public-pool.io:21496 · bc1qar0s…wf5mdq.bitaxe
  Dashboard  https://web.public-pool.io/#/app/bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq
○ Fallback   solo.ckpool.org:3333 · bc1qar0s…wf5mdq.bitaxe
"
        ));
    }

    #[test]
    fn links_the_dashboard_on_firmware_without_pool_slots() {
        let mut info = oldest();
        info.stratum_url = "mine.ocean.xyz".to_string();
        info.stratum_user = "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".to_string();

        let output = render("192.168.1.42", &info, false);

        assert!(output.ends_with(
            "\
● Primary    mine.ocean.xyz:21496 · 1A1zP1eP…DivfNa
  Dashboard  https://ocean.xyz/stats/1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa
"
        ));
    }

    #[test]
    fn color_only_adds_escape_codes() {
        let mut info = latest();
        info.temp = 80.0;

        let plain = render("192.168.1.42", &info, false);
        let colored = render("192.168.1.42", &info, true);

        assert!(!plain.contains('\u{1b}'));
        assert!(colored.contains("\u{1b}[1m\u{1b}[31m80 °C\u{1b}[0m"));

        let mut stripped = String::new();
        let mut chars = colored.chars();
        while let Some(c) = chars.next() {
            if c == '\u{1b}' {
                chars.by_ref().find(|c| *c == 'm');
            } else {
                stripped.push(c);
            }
        }
        assert_eq!(stripped, plain);
    }

    #[test]
    fn scales_hashrate_units() {
        assert_eq!(hashrate(0.0), "0 GH/s");
        assert_eq!(hashrate(0.4567), "456.7 MH/s");
        assert_eq!(hashrate(503.21), "503.2 GH/s");
        assert_eq!(hashrate(1103.4567), "1.10 TH/s");
    }

    #[test]
    fn formats_difficulty_from_either_representation() {
        assert_eq!(difficulty(&StringOrInt::String("4.29G".into())), "4.29G");
        assert_eq!(difficulty(&StringOrInt::Int(4_294_967_296)), "4.29G");
        assert_eq!(difficulty(&StringOrInt::Float(1_500_000.0)), "1.5M");
        assert_eq!(difficulty(&StringOrInt::Int(512)), "512");
    }

    #[test]
    fn groups_thousands() {
        assert_eq!(grouped(0), "0");
        assert_eq!(grouped(999), "999");
        assert_eq!(grouped(1_000), "1,000");
        assert_eq!(grouped(12_345_678), "12,345,678");
    }

    #[test]
    fn uptime_uses_largest_units() {
        assert_eq!(uptime(42), "42s");
        assert_eq!(uptime(3_600), "1h");
        assert_eq!(uptime(273_170), "3d 3h 52m");
    }

    #[test]
    fn shortens_long_pool_addresses_but_keeps_the_worker() {
        assert_eq!(shorten_user("bc1qexample.worker"), "bc1qexample.worker");
        assert_eq!(
            shorten_user("bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq.bitaxe"),
            "bc1qar0s…wf5mdq.bitaxe"
        );
    }
}
