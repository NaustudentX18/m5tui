//! `m5tui-status` — system status readings and the doctor self-check.
//!
//! Each reading is a small `*Status` struct with a `display()` method
//! for the cockpit bottom bar and a `healthy()` predicate the doctor
//! uses. The doctor walks every reading, scores it, and can export a
//! Markdown report to a path the framework supplies (typically
//! `/sd/m5tui/logs/doctor-<unix>.md`).
//!
//! The crate is `no_std`-free but alloc-only: it uses `String` for the
//! report body. Hardware-side readings (battery voltage, Wi-Fi RSSI)
//! are stubbed in this version and return synthetic values; the device
//! will replace the stub implementations with real I2C / sysfs reads
//! later.

#![allow(clippy::result_large_err)]

use std::fmt::Write as _;

/// One status row. The doctor renders every reading as a Markdown
/// bullet and tags the row as `OK`, `WARN`, or `FAIL` based on
/// `healthy()`.
pub trait Reading {
    fn name(&self) -> &'static str;
    fn value(&self) -> String;
    fn healthy(&self) -> StatusKind;
}

/// A simple OK / WARN / FAIL envelope the doctor uses for scoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    Ok,
    Warn,
    Fail,
}

impl StatusKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::Warn => "WARN",
            Self::Fail => "FAIL",
        }
    }

    pub fn weight(self) -> u32 {
        match self {
            Self::Ok => 0,
            Self::Warn => 1,
            Self::Fail => 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatteryStatus {
    pub percent: u8,
    pub charging: bool,
}

impl BatteryStatus {
    pub fn healthy(&self) -> StatusKind {
        if self.percent >= 30 {
            StatusKind::Ok
        } else if self.percent >= 10 {
            StatusKind::Warn
        } else {
            StatusKind::Fail
        }
    }

    pub fn display(&self) -> String {
        let charge = if self.charging { "+" } else { "-" };
        format!("{charge}{}%", self.percent)
    }
}

impl Reading for BatteryStatus {
    fn name(&self) -> &'static str {
        "battery"
    }
    fn value(&self) -> String {
        let mut s = self.display();
        if self.charging {
            s.push_str(" charging");
        }
        s
    }
    fn healthy(&self) -> StatusKind {
        self.healthy()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WifiStatus {
    pub ssid: String,
    pub rssi_dbm: i32,
}

impl WifiStatus {
    pub fn healthy(&self) -> StatusKind {
        if self.ssid.is_empty() {
            StatusKind::Fail
        } else if self.rssi_dbm >= -65 {
            StatusKind::Ok
        } else if self.rssi_dbm >= -80 {
            StatusKind::Warn
        } else {
            StatusKind::Fail
        }
    }
}

impl Reading for WifiStatus {
    fn name(&self) -> &'static str {
        "wifi"
    }
    fn value(&self) -> String {
        format!("{} ({} dBm)", self.ssid, self.rssi_dbm)
    }
    fn healthy(&self) -> StatusKind {
        self.healthy()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TailscaleStatus {
    pub up: bool,
    pub ip: String,
}

impl Reading for TailscaleStatus {
    fn name(&self) -> &'static str {
        "tailscale"
    }
    fn value(&self) -> String {
        if self.up {
            format!("up ({})", self.ip)
        } else {
            "down".to_string()
        }
    }
    fn healthy(&self) -> StatusKind {
        if self.up {
            StatusKind::Ok
        } else {
            StatusKind::Fail
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerHealth {
    pub host: String,
    pub ping_ms: Option<u32>,
}

impl Reading for ServerHealth {
    fn name(&self) -> &'static str {
        "server"
    }
    fn value(&self) -> String {
        match self.ping_ms {
            Some(ms) => format!("{} ping {} ms", self.host, ms),
            None => format!("{} timeout", self.host),
        }
    }
    fn healthy(&self) -> StatusKind {
        match self.ping_ms {
            Some(ms) if ms <= 50 => StatusKind::Ok,
            Some(_) => StatusKind::Warn,
            None => StatusKind::Fail,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmpPing {
    pub model: String,
    pub ping_ms: Option<u32>,
}

impl Reading for OmpPing {
    fn name(&self) -> &'static str {
        "omp"
    }
    fn value(&self) -> String {
        match self.ping_ms {
            Some(ms) => format!("{} ping {} ms", self.model, ms),
            None => format!("{} timeout", self.model),
        }
    }
    fn healthy(&self) -> StatusKind {
        match self.ping_ms {
            Some(_) => StatusKind::Ok,
            None => StatusKind::Fail,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiskUsage {
    pub free_gib: f32,
    pub total_gib: f32,
}

impl Reading for DiskUsage {
    fn name(&self) -> &'static str {
        "disk"
    }
    fn value(&self) -> String {
        format!("{:.1} / {:.1} GiB free", self.free_gib, self.total_gib)
    }
    fn healthy(&self) -> StatusKind {
        if self.free_gib >= 1.0 {
            StatusKind::Ok
        } else if self.free_gib >= 0.25 {
            StatusKind::Warn
        } else {
            StatusKind::Fail
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Uptime {
    pub seconds: u64,
}

impl Reading for Uptime {
    fn name(&self) -> &'static str {
        "uptime"
    }
    fn value(&self) -> String {
        let d = self.seconds / 86_400;
        let h = (self.seconds / 3_600) % 24;
        let m = (self.seconds / 60) % 60;
        format!("{d}d {h}h {m}m")
    }
    fn healthy(&self) -> StatusKind {
        StatusKind::Ok
    }
}

/// A snapshot of every reading the doctor evaluates. The `started_at`
/// is a unix timestamp; the doctor stamps every report with it.
#[derive(Debug, Clone, PartialEq)]
pub struct DoctorSnapshot {
    pub started_at: u64,
    pub battery: BatteryStatus,
    pub wifi: WifiStatus,
    pub tailscale: TailscaleStatus,
    pub server: ServerHealth,
    pub omp: OmpPing,
    pub disk: DiskUsage,
    pub uptime: Uptime,
}

impl DoctorSnapshot {
    /// Compute the weighted score: 0 = perfect, larger = worse.
    pub fn score(&self) -> u32 {
        let mut s = 0u32;
        for r in self.readings() {
            s += r.healthy().weight();
        }
        s
    }

    /// Aggregate verdict: `OK` if every reading is OK, `WARN` if at
    /// least one warning and no failures, `FAIL` if any reading is
    /// failing.
    pub fn verdict(&self) -> StatusKind {
        let mut has_warn = false;
        for r in self.readings() {
            match r.healthy() {
                StatusKind::Fail => return StatusKind::Fail,
                StatusKind::Warn => has_warn = true,
                StatusKind::Ok => {}
            }
        }
        if has_warn {
            StatusKind::Warn
        } else {
            StatusKind::Ok
        }
    }

    /// Borrow the readings in a stable order.
    pub fn readings(&self) -> Vec<&dyn Reading> {
        vec![
            &self.battery,
            &self.wifi,
            &self.tailscale,
            &self.server,
            &self.omp,
            &self.disk,
            &self.uptime,
        ]
    }

    /// Render the snapshot as a Markdown report. The output is intended
    /// for `~/.m5tui/logs/doctor-<unix>.md` and is self-contained.
    pub fn render_markdown(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "# m5Tui Doctor Report");
        let _ = writeln!(out);
        let _ = writeln!(out, "- started_at: {}", self.started_at);
        let _ = writeln!(out, "- verdict: **{}**", self.verdict().label());
        let _ = writeln!(out, "- score: {}", self.score());
        let _ = writeln!(out);
        let _ = writeln!(out, "| reading | value | status |");
        let _ = writeln!(out, "| ------- | ----- | ------ |");
        for r in self.readings() {
            let _ = writeln!(
                out,
                "| {} | {} | {} |",
                r.name(),
                r.value(),
                r.healthy().label()
            );
        }
        out
    }

    /// Default snapshot with synthetic values, useful for the
    /// "press D" demo path. Real reads happen in `doctor_run` on the
    /// device.
    pub fn demo() -> Self {
        Self {
            started_at: 1_718_500_000,
            battery: BatteryStatus {
                percent: 78,
                charging: false,
            },
            wifi: WifiStatus {
                ssid: "aiserver-5g".into(),
                rssi_dbm: -54,
            },
            tailscale: TailscaleStatus {
                up: true,
                ip: "100.127.91.97".into(),
            },
            server: ServerHealth {
                host: "aiserver-1".into(),
                ping_ms: Some(12),
            },
            omp: OmpPing {
                model: "qwen3-14b".into(),
                ping_ms: Some(18),
            },
            disk: DiskUsage {
                free_gib: 2.1,
                total_gib: 14.9,
            },
            uptime: Uptime {
                seconds: 2 * 86_400 + 4 * 3_600 + 17 * 60,
            },
        }
    }
}

/// Default log filename for the doctor report. Caller appends the unix
/// timestamp; the framework persists to `/sd/m5tui/logs/<name>`.
pub fn report_filename(started_at: u64) -> String {
    format!("doctor-{started_at}.md")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battery_healthy_thresholds() {
        assert_eq!(
            BatteryStatus {
                percent: 90,
                charging: false
            }
            .healthy(),
            StatusKind::Ok
        );
        assert_eq!(
            BatteryStatus {
                percent: 30,
                charging: false
            }
            .healthy(),
            StatusKind::Ok
        );
        assert_eq!(
            BatteryStatus {
                percent: 20,
                charging: false
            }
            .healthy(),
            StatusKind::Warn
        );
        assert_eq!(
            BatteryStatus {
                percent: 5,
                charging: false
            }
            .healthy(),
            StatusKind::Fail
        );
    }

    #[test]
    fn wifi_healthy_thresholds() {
        assert_eq!(
            WifiStatus {
                ssid: "x".into(),
                rssi_dbm: -50
            }
            .healthy(),
            StatusKind::Ok
        );
        assert_eq!(
            WifiStatus {
                ssid: "x".into(),
                rssi_dbm: -70
            }
            .healthy(),
            StatusKind::Warn
        );
        assert_eq!(
            WifiStatus {
                ssid: "x".into(),
                rssi_dbm: -90
            }
            .healthy(),
            StatusKind::Fail
        );
        assert_eq!(
            WifiStatus {
                ssid: "".into(),
                rssi_dbm: -50
            }
            .healthy(),
            StatusKind::Fail
        );
    }

    #[test]
    fn tailscale_healthy() {
        assert_eq!(
            TailscaleStatus {
                up: true,
                ip: "1.2.3.4".into()
            }
            .healthy(),
            StatusKind::Ok
        );
        assert_eq!(
            TailscaleStatus {
                up: false,
                ip: "".into()
            }
            .healthy(),
            StatusKind::Fail
        );
    }

    #[test]
    fn server_healthy_thresholds() {
        assert_eq!(
            ServerHealth {
                host: "h".into(),
                ping_ms: Some(10)
            }
            .healthy(),
            StatusKind::Ok
        );
        assert_eq!(
            ServerHealth {
                host: "h".into(),
                ping_ms: Some(200)
            }
            .healthy(),
            StatusKind::Warn
        );
        assert_eq!(
            ServerHealth {
                host: "h".into(),
                ping_ms: None
            }
            .healthy(),
            StatusKind::Fail
        );
    }

    #[test]
    fn disk_healthy_thresholds() {
        assert_eq!(
            DiskUsage {
                free_gib: 5.0,
                total_gib: 10.0
            }
            .healthy(),
            StatusKind::Ok
        );
        assert_eq!(
            DiskUsage {
                free_gib: 0.5,
                total_gib: 10.0
            }
            .healthy(),
            StatusKind::Warn
        );
        assert_eq!(
            DiskUsage {
                free_gib: 0.1,
                total_gib: 10.0
            }
            .healthy(),
            StatusKind::Fail
        );
    }

    #[test]
    fn snapshot_score_sums_weights() {
        let s = DoctorSnapshot::demo();
        assert_eq!(s.verdict(), StatusKind::Ok);
        assert_eq!(s.score(), 0);
    }

    #[test]
    fn snapshot_verdict_fails_on_any_fail() {
        let mut s = DoctorSnapshot::demo();
        s.battery.percent = 1;
        assert_eq!(s.verdict(), StatusKind::Fail);
        assert!(s.score() >= 3);
    }

    #[test]
    fn snapshot_verdict_warn_when_only_warnings() {
        let mut s = DoctorSnapshot::demo();
        s.disk.free_gib = 0.5;
        assert_eq!(s.verdict(), StatusKind::Warn);
    }

    #[test]
    fn render_markdown_contains_readings() {
        let md = DoctorSnapshot::demo().render_markdown();
        for name in [
            "battery",
            "wifi",
            "tailscale",
            "server",
            "omp",
            "disk",
            "uptime",
        ] {
            assert!(md.contains(name), "missing reading '{name}'");
        }
        assert!(md.contains("verdict"));
    }

    #[test]
    fn report_filename_uses_unix() {
        assert_eq!(report_filename(123), "doctor-123.md");
    }

    #[test]
    fn status_kind_weight() {
        assert_eq!(StatusKind::Ok.weight(), 0);
        assert_eq!(StatusKind::Warn.weight(), 1);
        assert_eq!(StatusKind::Fail.weight(), 3);
    }

    #[test]
    fn readings_have_stable_order() {
        let s = DoctorSnapshot::demo();
        let names: Vec<&'static str> = s.readings().iter().map(|r| r.name()).collect();
        assert_eq!(
            names,
            vec![
                "battery",
                "wifi",
                "tailscale",
                "server",
                "omp",
                "disk",
                "uptime"
            ]
        );
    }
}
