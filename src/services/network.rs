use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::thread;

#[derive(Debug, Clone)]
pub struct WifiNetwork {
    pub ssid: String,
    pub signal: u32,
    pub security: String,
    pub is_connected: bool,
}

pub struct NetworkService;

impl NetworkService {
    /// Check if Wi-Fi radio is enabled
    pub fn is_wifi_enabled() -> bool {
        if let Ok(output) = Command::new("nmcli").args(["radio", "wifi"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            return stdout.trim().eq_ignore_ascii_case("enabled");
        }
        true
    }

    /// Toggle Wi-Fi radio ON/OFF
    pub fn set_wifi_enabled(enabled: bool) {
        let arg = if enabled { "on" } else { "off" };
        let _ = Command::new("nmcli").args(["radio", "wifi", arg]).output();
    }

    /// Get currently connected Wi-Fi SSID
    pub fn get_active_ssid() -> Option<String> {
        if let Ok(output) = Command::new("nmcli")
            .args(["-t", "-f", "ACTIVE,SSID", "dev", "wifi"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.starts_with("yes:") {
                    let ssid = &line["yes:".len()..];
                    if !ssid.is_empty() {
                        return Some(ssid.to_string());
                    }
                }
            }
        }
        None
    }

    /// Scan nearby Wi-Fi access points
    pub fn scan_networks() -> Vec<WifiNetwork> {
        let mut networks: Vec<WifiNetwork> = Vec::new();

        if let Ok(output) = Command::new("nmcli")
            .args(["-t", "-f", "SSID,SIGNAL,SECURITY,ACTIVE", "dev", "wifi", "list", "--rescan", "yes"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 4 {
                    let ssid = parts[0].trim().to_string();
                    if ssid.is_empty() {
                        continue;
                    }

                    // Avoid duplicate SSIDs
                    if networks.iter().any(|n| n.ssid == ssid) {
                        continue;
                    }

                    let signal = parts[1].trim().parse::<u32>().unwrap_or(50);
                    let security = parts[2].trim().to_string();
                    let is_connected = parts[3].trim().eq_ignore_ascii_case("yes");

                    networks.push(WifiNetwork {
                        ssid,
                        signal,
                        security,
                        is_connected,
                    });
                }
            }
        }

        networks
    }

    /// Connect to Wi-Fi network by SSID
    pub fn connect_network(ssid: &str, password: Option<&str>) {
        let s = ssid.to_string();
        let p = password.map(|str_ref| str_ref.to_string());
        thread::spawn(move || {
            let mut cmd = Command::new("nmcli");
            cmd.args(["dev", "wifi", "connect", &s]);
            if let Some(pass) = p {
                cmd.args(["password", &pass]);
            }
            let _ = cmd.output();
        });
    }

    /// Event stream listener via `nmcli monitor` for zero-polling network updates
    pub fn listen_events<F>(mut callback: F)
    where
        F: FnMut() + Send + 'static,
    {
        thread::spawn(move || {
            if let Ok(mut child) = Command::new("nmcli")
                .arg("monitor")
                .stdout(Stdio::piped())
                .spawn()
            {
                if let Some(stdout) = child.stdout.take() {
                    let reader = BufReader::new(stdout);
                    for _ in reader.lines().flatten() {
                        callback();
                    }
                }
            }
        });
    }
}
