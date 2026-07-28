use crate::services::worker::TaskWorker;
use std::process::Command;

pub struct BrightnessService;

impl BrightnessService {
    /// Get current brightness percentage (0..100) using direct sysfs read with CLI fallback
    pub fn get_brightness() -> u32 {
        // Direct sysfs read (0 process forks, < 0.01 ms execution)
        if let Ok(entries) = std::fs::read_dir("/sys/class/backlight") {
            for entry in entries.flatten() {
                let path = entry.path();
                let curr_path = path.join("brightness");
                let max_path = path.join("max_brightness");

                if let (Ok(curr_str), Ok(max_str)) = (
                    std::fs::read_to_string(&curr_path),
                    std::fs::read_to_string(&max_path),
                ) {
                    if let (Ok(curr), Ok(max)) = (
                        curr_str.trim().parse::<f32>(),
                        max_str.trim().parse::<f32>(),
                    ) {
                        if max > 0.0 {
                            return ((curr / max) * 100.0).round() as u32;
                        }
                    }
                }
            }
        }

        // Fallback via brightnessctl CLI if sysfs is unreadable
        if let Ok(output) = Command::new("brightnessctl")
            .env("LC_ALL", "C")
            .arg("g")
            .output()
        {
            let curr: f32 = String::from_utf8_lossy(&output.stdout)
                .trim()
                .parse()
                .unwrap_or(0.0);

            if let Ok(max_output) = Command::new("brightnessctl")
                .env("LC_ALL", "C")
                .arg("m")
                .output()
            {
                let max: f32 = String::from_utf8_lossy(&max_output.stdout)
                    .trim()
                    .parse()
                    .unwrap_or(1.0);

                if max > 0.0 {
                    return ((curr / max) * 100.0).round() as u32;
                }
            }
        }
        100
    }

    /// Set brightness percentage (0..100) asynchronously using persistent worker thread
    pub fn set_brightness(pct: u32) {
        let val_str = format!("{pct}%");
        TaskWorker::dispatch(move || {
            let _ = Command::new("brightnessctl")
                .env("LC_ALL", "C")
                .args(["set", &val_str])
                .output();
        });
    }
}
