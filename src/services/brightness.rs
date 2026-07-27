use crate::services::worker::TaskWorker;
use std::process::Command;

pub struct BrightnessService;

impl BrightnessService {
    /// Get current brightness percentage (0..100)
    pub fn get_brightness() -> u32 {
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
