use std::process::Command;

pub struct NightLightService;

impl NightLightService {
    /// Check if night light (gammastep / wlsunset / hyprshade) is currently running
    pub fn is_enabled() -> bool {
        let targets = ["gammastep", "wlsunset", "hyprshade"];
        for proc in &targets {
            if let Ok(output) = Command::new("pgrep")
                .env("LC_ALL", "C")
                .args(["-x", proc])
                .output()
            {
                if output.status.success() {
                    return true;
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
