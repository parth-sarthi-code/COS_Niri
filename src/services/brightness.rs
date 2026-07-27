use std::process::Command;

pub struct BrightnessService;

impl BrightnessService {
    /// Get screen brightness percentage (0..100)
    pub fn get_brightness() -> u32 {
        if let Ok(output) = Command::new("brightnessctl").arg("g").output() {
            let curr = String::from_utf8_lossy(&output.stdout)
                .trim()
                .parse::<f32>()
                .unwrap_or(0.0);

            if let Ok(max_out) = Command::new("brightnessctl").arg("m").output() {
                let max = String::from_utf8_lossy(&max_out.stdout)
                    .trim()
                    .parse::<f32>()
                    .unwrap_or(1.0);
                if max > 0.0 {
                    return ((curr / max) * 100.0).round() as u32;
                }
            }
        }
        100
    }

    /// Set screen brightness percentage (0..100)
    pub fn set_brightness(pct: u32) {
        let val_str = format!("{pct}%");
        let _ = Command::new("brightnessctl").arg("set").arg(val_str).output();
    }
}
