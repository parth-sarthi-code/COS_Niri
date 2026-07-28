use crate::services::worker::TaskWorker;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PowerProfile {
    Performance,
    Balanced,
    PowerSaver,
}

impl PowerProfile {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Performance => "Performance",
            Self::Balanced => "Balanced",
            Self::PowerSaver => "Battery saver",
        }
    }

    pub fn gnome_icon_name(&self) -> &'static str {
        match self {
            Self::Performance => "power-profile-performance-symbolic",
            Self::Balanced => "power-profile-balanced-symbolic",
            Self::PowerSaver => "power-profile-power-saver-symbolic",
        }
    }

    pub fn icon_code(&self) -> &'static str {
        match self {
            Self::Performance => "\u{e80e}", // speed gauge (GNOME style)
            Self::Balanced => "\u{e429}",    // dial / sliders (GNOME style)
            Self::PowerSaver => "\u{e80d}",  // leaf (GNOME style)
        }
    }

    pub fn to_cmd_str(&self) -> &'static str {
        match self {
            Self::Performance => "performance",
            Self::Balanced => "balanced",
            Self::PowerSaver => "power-saver",
        }
    }
}

pub struct PowerProfileService;

impl PowerProfileService {
    /// Get current active power profile
    pub fn get_profile() -> PowerProfile {
        if let Ok(output) = Command::new("powerprofilesctl")
            .env("LC_ALL", "C")
            .arg("get")
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_lowercase();
            if stdout.contains("performance") {
                return PowerProfile::Performance;
            } else if stdout.contains("power-saver") {
                return PowerProfile::PowerSaver;
            }
        }
        PowerProfile::Balanced
    }

    /// Cycle to the next power profile (Balanced -> Performance -> PowerSaver -> Balanced)
    pub fn cycle_profile() -> PowerProfile {
        let current = Self::get_profile();
        let next = match current {
            PowerProfile::Balanced => PowerProfile::Performance,
            PowerProfile::Performance => PowerProfile::PowerSaver,
            PowerProfile::PowerSaver => PowerProfile::Balanced,
        };

        let cmd_arg = next.to_cmd_str();
        TaskWorker::dispatch(move || {
            let _ = Command::new("powerprofilesctl")
                .env("LC_ALL", "C")
                .args(["set", cmd_arg])
                .output();
        });

        next
    }

    /// Set active power profile asynchronously
    pub fn set_profile(profile: PowerProfile) {
        let cmd_arg = profile.to_cmd_str();
        TaskWorker::dispatch(move || {
            let _ = Command::new("powerprofilesctl")
                .env("LC_ALL", "C")
                .args(["set", cmd_arg])
                .output();
        });
    }
}
