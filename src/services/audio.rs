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
    /// Get volume AND mute state from a single `wpctl get-volume` call (1 fork instead of 2)
    pub fn get_volume_and_mute() -> (u32, bool) {
        if let Ok(output) = Command::new("wpctl")
            .env("LC_ALL", "C")
            .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let is_muted = stdout.contains("[MUTED]");
            for word in stdout.split_whitespace() {
                if let Ok(val) = word.parse::<f32>() {
                    return ((val * 100.0).round() as u32, is_muted);
                }
            }
            return (50, is_muted);
        }
        (50, false)
    }

    /// Get current volume percentage (0..100) of default sink
    pub fn get_volume() -> u32 {
        Self::get_volume_and_mute().0
    }

    /// Check if default sink is muted
    pub fn is_muted() -> bool {
        Self::get_volume_and_mute().1
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

    /// Toggle mute asynchronously using single persistent worker thread
    pub fn toggle_mute() {
        crate::services::worker::TaskWorker::dispatch(move || {
            let _ = Command::new("wpctl")
                .env("LC_ALL", "C")
                .args(["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"])
                .output();
        });
    }

    /// Get list of connected audio output sinks (single `pactl list sinks` fork)
    pub fn get_sinks() -> Vec<AudioSink> {
        let mut sinks = Vec::new();

        if let Ok(output) = Command::new("pactl")
            .env("LC_ALL", "C")
            .args(["list", "sinks"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut current_name = String::new();
            let mut current_is_running = false;

            for line in stdout.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("Name: ") {
                    current_name = trimmed["Name: ".len()..].to_string();
                    current_is_running = false;
                } else if trimmed.starts_with("State: ") {
                    current_is_running = trimmed.contains("RUNNING");
                } else if trimmed.starts_with("Description: ") {
                    let raw_desc = &trimmed["Description: ".len()..];
                    let desc = Self::clean_device_name(if raw_desc.is_empty() { &current_name } else { raw_desc });
                    sinks.push(AudioSink {
                        name: current_name.clone(),
                        description: desc,
                        is_default: current_is_running,
                    });
                }
            }

            // If no sink is marked RUNNING, fall back to pactl get-default-sink
            if !sinks.is_empty() && !sinks.iter().any(|s| s.is_default) {
                let default_name = Self::get_default_sink_name();
                for sink in &mut sinks {
                    if sink.name == default_name {
                        sink.is_default = true;
                        break;
                    }
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

    /// Clean up redundant hardware chipset prefixes — zero-alloc until final .to_string()
    fn clean_device_name(raw_desc: &str) -> String {
        let mut desc = raw_desc.trim();

        let prefixes = [
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

        for prefix in &prefixes {
            if let Some(stripped) = desc.strip_prefix(prefix) {
                desc = stripped;
                break; // Only strip first matching prefix
            }
        }

        if let Some(stripped) = desc.strip_suffix(" Output") {
            desc = stripped;
        }

        if desc.eq_ignore_ascii_case("speaker") {
            return "Speakers".to_string();
        }

        if desc.is_empty() {
            return raw_desc.to_string();
        }

        desc.to_string()
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

    /// Set active audio output sink asynchronously using persistent worker thread (PipeWire + PulseAudio)
    pub fn set_default_sink(sink_name: &str) {
        let name = sink_name.to_string();
        crate::services::worker::TaskWorker::dispatch(move || {
            // 1. PulseAudio compatibility fallback
            let _ = Command::new("pactl")
                .env("LC_ALL", "C")
                .args(["set-default-sink", &name])
                .output();

            // 2. WirePlumber / PipeWire native stream update
            let _ = Command::new("wpctl")
                .env("LC_ALL", "C")
                .args(["set-default", &name])
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
