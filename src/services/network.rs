use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::thread;

#[derive(Debug, Clone)]
pub struct WifiNetwork {
    pub ssid: String,
    pub signal: u32,
    pub is_connected: bool,
}

pub struct NetworkService;

impl NetworkService {
    /// Check if Wi-Fi interface radio is enabled
    pub fn is_wifi_enabled() -> bool {
        if let Ok(output) = Command::new("nmcli")
            .env("LC_ALL", "C")
            .args(["radio", "wifi"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            return stdout.trim().eq_ignore_ascii_case("enabled");
        }
        false
    }

    /// Toggle Wi-Fi power ON/OFF asynchronously using worker thread
    pub fn set_wifi_enabled(enabled: bool) {
        let arg = if enabled { "on" } else { "off" };
        crate::services::worker::TaskWorker::dispatch(move || {
            let _ = Command::new("nmcli")
                .env("LC_ALL", "C")
                .args(["radio", "wifi", arg])
                .output();
        });
    }

    /// Get currently active connected Wi-Fi SSID
    pub fn get_active_ssid() -> Option<String> {
        if let Ok(output) = Command::new("nmcli")
            .env("LC_ALL", "C")
            .args(["-t", "-f", "ACTIVE,SSID", "dev", "wifi"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.starts_with("yes:") {
                    let ssid = line["yes:".len()..].trim().to_string();
                    if !ssid.is_empty() {
                        return Some(ssid);
                    }
                }
            }
        }
        None
    }

    /// Scan available Wi-Fi networks
    pub fn scan_networks() -> Vec<WifiNetwork> {
        let mut networks = Vec::new();

        if let Ok(output) = Command::new("nmcli")
            .env("LC_ALL", "C")
            .args(["-t", "-f", "SSID,SIGNAL,ACTIVE", "dev", "wifi", "list"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                // Line format: "SSID:SIGNAL:ACTIVE" (escaping colons)
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 3 {
                    let ssid = parts[0].trim().to_string();
                    let signal = parts[1].parse::<u32>().unwrap_or(0);
                    let is_connected = parts[2].eq_ignore_ascii_case("yes");

                    if !ssid.is_empty() && !networks.iter().any(|n: &WifiNetwork| n.ssid == ssid) {
                        networks.push(WifiNetwork {
                            ssid,
                            signal,
                            is_connected,
                        });
                    }
                }
            }
        }

        networks
    }

    /// Connect to a Wi-Fi network asynchronously using worker thread
    pub fn connect_network(ssid: &str, password: Option<&str>) {
        let s = ssid.to_string();
        let p = password.map(|pass| pass.to_string());
        crate::services::worker::TaskWorker::dispatch(move || {
            let mut cmd = Command::new("nmcli");
            cmd.env("LC_ALL", "C");
            cmd.args(["device", "wifi", "connect", &s]);
            if let Some(pass) = p {
                cmd.args(["password", &pass]);
            }
            let _ = cmd.output();
        });
    }

    /// Event stream listener via `nmcli monitor` for sub-second zero-polling network updates
    pub fn listen_events<F>(mut callback: F)
    where
        F: FnMut() + Send + 'static,
    {
        thread::spawn(move || {
            if let Ok(mut child) = Command::new("nmcli")
                .env("LC_ALL", "C")
                .arg("monitor")
                .stdout(Stdio::piped())
                .spawn()
            {
                if let Some(stdout) = child.stdout.take() {
                    let reader = BufReader::new(stdout);
                    for _line in reader.lines().flatten() {
                        callback();
                    }
                }
            }
        });
    }
}
