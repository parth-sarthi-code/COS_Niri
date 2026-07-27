use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::thread;

#[derive(Debug, Clone)]
pub struct AudioSink {
    pub name: String,
    pub description: String,
    pub is_default: bool,
}

pub struct AudioService;

impl AudioService {
    /// Get current volume percentage (0..100) of default sink
    pub fn get_volume() -> u32 {
        if let Ok(output) = Command::new("wpctl")
            .env("LC_ALL", "C")
            .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for word in stdout.split_whitespace() {
                if let Ok(val) = word.parse::<f32>() {
                    return (val * 100.0).round() as u32;
                }
            }
        }
        // Fallback via pactl
        if let Ok(output) = Command::new("pactl")
            .env("LC_ALL", "C")
            .args(["get-sink-volume", "@DEFAULT_SINK@"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for word in stdout.split_whitespace() {
                if word.ends_with('%') {
                    if let Ok(val) = word.trim_end_matches('%').parse::<u32>() {
                        return val;
                    }
                }
            }
        }
        50
    }

    /// Set volume percentage (0..100) asynchronously using single persistent worker thread
    pub fn set_volume(pct: u32) {
        let val_str = format!("{pct}%");
        crate::services::worker::TaskWorker::dispatch(move || {
            let _ = Command::new("wpctl")
                .env("LC_ALL", "C")
                .args(["set-volume", "@DEFAULT_AUDIO_SINK@", &val_str])
                .output();
        });
    }

    /// Check if default sink is muted
    pub fn is_muted() -> bool {
        if let Ok(output) = Command::new("wpctl")
            .env("LC_ALL", "C")
            .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            return stdout.contains("[MUTED]");
        }
        false
    }

    /// Toggle mute asynchronously using single persistent worker thread
    pub fn toggle_mute() {
        crate::services::worker::TaskWorker::dispatch(move || {
            let _ = Command::new("wpctl")
                .env("LC_ALL", "C")
                .args(["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"])
                .output();
        });
    }

    /// Get list of connected audio output sinks
    pub fn get_sinks() -> Vec<AudioSink> {
        let mut sinks = Vec::new();
        let default_sink = Self::get_default_sink_name();

        if let Ok(output) = Command::new("pactl")
            .env("LC_ALL", "C")
            .args(["list", "sinks"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut current_name = String::new();

            for line in stdout.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("Name: ") {
                    current_name = trimmed["Name: ".len()..].to_string();
                } else if trimmed.starts_with("Description: ") {
                    let raw_desc = trimmed["Description: ".len()..].to_string();
                    let desc = Self::clean_device_name(if raw_desc.is_empty() { &current_name } else { &raw_desc });
                    let is_default = current_name == default_sink;
                    sinks.push(AudioSink {
                        name: current_name.clone(),
                        description: desc,
                        is_default,
                    });
                }
            }
        }

        if sinks.is_empty() {
            sinks.push(AudioSink {
                name: "default".into(),
                description: "Built-in Speakers".into(),
                is_default: true,
            });
        }

        sinks
    }

    /// Clean up redundant hardware chipset prefixes and noisy suffixes from audio descriptions
    fn clean_device_name(raw_desc: &str) -> String {
        let mut desc = raw_desc.trim().to_string();

        let prefixes_to_remove = [
            "700 Series Chipset Family HD Audio Controller ",
            "600 Series Chipset Family HD Audio Controller ",
            "500 Series Chipset Family HD Audio Controller ",
            "400 Series Chipset Family HD Audio Controller ",
            "300 Series Chipset Family HD Audio Controller ",
            "200 Series Chipset Family HD Audio Controller ",
            "100 Series Chipset Family HD Audio Controller ",
            "Family HD Audio Controller ",
            "High Definition Audio Controller ",
            "HD Audio Controller ",
            "Built-in Audio ",
            "Intel Corporation ",
            "Advanced Micro Devices, Inc. [AMD] ",
            "NVIDIA Corporation ",
            "Audio Controller ",
        ];

        for prefix in &prefixes_to_remove {
            if desc.starts_with(prefix) {
                desc = desc[prefix.len()..].to_string();
            }
        }

        if desc.ends_with(" Output") {
            desc = desc[..desc.len() - " Output".len()].to_string();
        }

        if desc.eq_ignore_ascii_case("speaker") {
            desc = "Speakers".to_string();
        }

        if desc.is_empty() {
            return raw_desc.to_string();
        }

        desc
    }

    fn get_default_sink_name() -> String {
        if let Ok(output) = Command::new("pactl")
            .env("LC_ALL", "C")
            .args(["get-default-sink"])
            .output()
        {
            return String::from_utf8_lossy(&output.stdout).trim().to_string();
        }
        String::new()
    }

    /// Set active audio output sink asynchronously using persistent worker thread
    pub fn set_default_sink(sink_name: &str) {
        let name = sink_name.to_string();
        crate::services::worker::TaskWorker::dispatch(move || {
            let _ = Command::new("pactl")
                .env("LC_ALL", "C")
                .args(["set-default-sink", &name])
                .output();
        });
    }

    /// Event stream listener via `pactl subscribe` for sub-second zero-polling UI updates
    pub fn listen_events<F>(mut callback: F)
    where
        F: FnMut() + Send + 'static,
    {
        thread::spawn(move || {
            if let Ok(mut child) = Command::new("pactl")
                .env("LC_ALL", "C")
                .arg("subscribe")
                .stdout(Stdio::piped())
                .spawn()
            {
                if let Some(stdout) = child.stdout.take() {
                    let reader = BufReader::new(stdout);
                    for line in reader.lines().flatten() {
                        if line.contains("sink") || line.contains("server") {
                            callback();
                        }
                    }
                }
            }
        });
    }
}
