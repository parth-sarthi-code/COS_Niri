use std::process::Command;

pub struct NightLightService;

impl NightLightService {
    /// Check if night light (gammastep / wlsunset / hyprshade) is currently running (0 process forks)
    pub fn is_enabled() -> bool {
        let targets = ["gammastep", "wlsunset", "hyprshade"];

        if let Ok(entries) = std::fs::read_dir("/proc") {
            for entry in entries.flatten() {
                let path = entry.path();
                // Check if directory name is numeric PID
                if path.is_dir() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.chars().all(|c| c.is_ascii_digit()) {
                            let comm_path = path.join("comm");
                            if let Ok(comm) = std::fs::read_to_string(&comm_path) {
                                let proc_name = comm.trim();
                                if targets.contains(&proc_name) {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
        false
    }

    /// Toggle night light state. Returns new active state (true = ON, false = OFF)
    pub fn toggle() -> bool {
        let currently_on = Self::is_enabled();
        if currently_on {
            let _ = Command::new("pkill")
                .env("LC_ALL", "C")
                .args(["-x", "gammastep"])
                .output();
            let _ = Command::new("pkill")
                .env("LC_ALL", "C")
                .args(["-x", "wlsunset"])
                .output();
            let _ = Command::new("pkill")
                .env("LC_ALL", "C")
                .args(["-x", "hyprshade"])
                .output();
            false
        } else {
            let _ = Command::new("sh")
                .env("LC_ALL", "C")
                .arg("-c")
                .arg("(gammastep -O 4500 || wlsunset -t 4500) >/dev/null 2>&1 &")
                .spawn();
            true
        }
    }
}
