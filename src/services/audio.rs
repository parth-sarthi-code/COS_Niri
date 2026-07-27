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
            .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Output format: "Volume: 0.65 [MUTED]" or "Volume: 0.65"
            for word in stdout.split_whitespace() {
                if let Ok(val) = word.parse::<f32>() {
                    return (val * 100.0).round() as u32;
                }
            }
        }
        // Fallback via pactl
        if let Ok(output) = Command::new("pactl")
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

    /// Set volume percentage (0..100) of default sink
    pub fn set_volume(pct: u32) {
        let val_str = format!("{pct}%");
        let _ = Command::new("wpctl")
            .args(["set-volume", "@DEFAULT_AUDIO_SINK@", &val_str])
            .output();
    }

    /// Check if default sink is muted
    pub fn is_muted() -> bool {
        if let Ok(output) = Command::new("wpctl")
            .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            return stdout.contains("[MUTED]");
        }
        false
    }

    /// Toggle mute on default sink
    pub fn toggle_mute() {
        let _ = Command::new("wpctl")
            .args(["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"])
            .output();
    }

    /// Get list of connected audio output sinks
    pub fn get_sinks() -> Vec<AudioSink> {
        let mut sinks = Vec::new();
        let default_sink = Self::get_default_sink_name();

        if let Ok(output) = Command::new("pactl").args(["list", "sinks"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut current_name = String::new();
            let mut current_desc = String::new();

            for line in stdout.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("Name: ") {
                    current_name = trimmed["Name: ".len()..].to_string();
                } else if trimmed.starts_with("Description: ") {
                    current_desc = trimmed["Description: ".len()..].to_string();
                    let is_default = current_name == default_sink;
                    sinks.push(AudioSink {
                        name: current_name.clone(),
                        description: if current_desc.is_empty() { current_name.clone() } else { current_desc.clone() },
                        is_default,
                    });
                }
            }
        }

        if sinks.is_empty() {
            sinks.push(AudioSink {
                name: "default".into(),
                description: "Built-in Audio".into(),
                is_default: true,
            });
        }

        sinks
    }

    fn get_default_sink_name() -> String {
        if let Ok(output) = Command::new("pactl").args(["get-default-sink"]).output() {
            return String::from_utf8_lossy(&output.stdout).trim().to_string();
        }
        String::new()
    }

    /// Set active audio output sink
    pub fn set_default_sink(sink_name: &str) {
        let _ = Command::new("pactl")
            .args(["set-default-sink", sink_name])
            .output();
    }

    /// Event stream listener via `pactl subscribe` for sub-second zero-polling UI updates
    pub fn listen_events<F>(mut callback: F)
    where
        F: FnMut() + Send + 'static,
    {
        thread::spawn(move || {
            if let Ok(mut child) = Command::new("pactl")
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
