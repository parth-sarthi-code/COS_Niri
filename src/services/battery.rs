use std::fs;
use std::process::Command;

pub struct BatteryInfo {
    pub capacity: u32,
    pub status: String,
    pub is_present: bool,
}

pub struct BatteryService;

impl BatteryService {
    /// Get current battery percentage and charging status
    pub fn get_info() -> BatteryInfo {
        let paths = [
            "/sys/class/power_supply/BAT0",
            "/sys/class/power_supply/BAT1",
            "/sys/class/power_supply/battery",
        ];

        for path in paths {
            let cap_path = format!("{path}/capacity");
            let stat_path = format!("{path}/status");

            if let Ok(cap_str) = fs::read_to_string(&cap_path) {
                if let Ok(cap) = cap_str.trim().parse::<u32>() {
                    let stat = fs::read_to_string(&stat_path)
                        .unwrap_or_else(|_| "Discharging".into())
                        .trim()
                        .to_string();

                    return BatteryInfo {
                        capacity: cap.min(100),
                        status: stat,
                        is_present: true,
                    };
                }
            }
        }

        // Fallback via upower CLI
        if let Ok(output) = Command::new("upower")
            .env("LC_ALL", "C")
            .args(["-i", "/org/freedesktop/UPower/devices/battery_BAT0"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut cap = None;
            let mut stat = "Discharging".to_string();

            for line in stdout.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("percentage:") {
                    if let Some(val_str) = trimmed.split_whitespace().nth(1) {
                        let num = val_str.trim_end_matches('%').parse::<u32>().unwrap_or(100);
                        cap = Some(num);
                    }
                } else if trimmed.starts_with("state:") {
                    if let Some(val_str) = trimmed.split_whitespace().nth(1) {
                        stat = val_str.to_string();
                    }
                }
            }

            if let Some(c) = cap {
                return BatteryInfo {
                    capacity: c.min(100),
                    status: stat,
                    is_present: true,
                };
            }
        }

        // Desktop PC fallback (no battery present)
        BatteryInfo {
            capacity: 100,
            status: "Full".into(),
            is_present: false,
        }
    }
}
