use gio::prelude::*;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::thread;

#[derive(Debug, Clone)]
pub struct WifiNetwork {
    pub ssid: String,
    pub signal: u32,
    pub is_connected: bool,
}

#[derive(Debug, Clone, Default)]
pub struct NetworkState {
    pub is_enabled: bool,
    pub ssid: Option<String>,
    pub signal: Option<u32>,
}

pub struct NetworkService;

impl NetworkService {
    /// Get unified NetworkManager state via native D-Bus IPC (0 process forks, <0.5ms)
    pub fn get_state() -> NetworkState {
        if let Ok(conn) = gio::bus_get_sync(gio::BusType::System, gio::Cancellable::NONE) {
            // Read WirelessEnabled property from org.freedesktop.NetworkManager
            let is_enabled = conn
                .call_sync(
                    Some("org.freedesktop.NetworkManager"),
                    "/org/freedesktop/NetworkManager",
                    "org.freedesktop.DBus.Properties",
                    "Get",
                    Some(&(
                        "org.freedesktop.NetworkManager",
                        "WirelessEnabled",
                    ).to_variant()),
                    None,
                    gio::DBusCallFlags::NONE,
                    -1,
                    gio::Cancellable::NONE,
                )
                .ok()
                .and_then(|res| res.child_value(0).get::<glib::Variant>())
                .and_then(|v| v.get::<bool>())
                .unwrap_or(false);

            if !is_enabled {
                return NetworkState {
                    is_enabled: false,
                    ssid: None,
                    signal: None,
                };
            }

            // Read PrimaryConnection object path
            let primary_path = conn
                .call_sync(
                    Some("org.freedesktop.NetworkManager"),
                    "/org/freedesktop/NetworkManager",
                    "org.freedesktop.DBus.Properties",
                    "Get",
                    Some(&(
                        "org.freedesktop.NetworkManager",
                        "PrimaryConnection",
                    ).to_variant()),
                    None,
                    gio::DBusCallFlags::NONE,
                    -1,
                    gio::Cancellable::NONE,
                )
                .ok()
                .and_then(|res| res.child_value(0).get::<glib::Variant>())
                .and_then(|v| v.get::<String>());

            let mut ssid = None;
            let mut signal = None;

            if let Some(ref path) = primary_path {
                if path != "/" && !path.is_empty() {
                    // Read SSID (Id property on ActiveConnection)
                    ssid = conn
                        .call_sync(
                            Some("org.freedesktop.NetworkManager"),
                            path,
                            "org.freedesktop.DBus.Properties",
                            "Get",
                            Some(&(
                                "org.freedesktop.NetworkManager.Connection.Active",
                                "Id",
                            ).to_variant()),
                            None,
                            gio::DBusCallFlags::NONE,
                            -1,
                            gio::Cancellable::NONE,
                        )
                        .ok()
                        .and_then(|res| res.child_value(0).get::<glib::Variant>())
                        .and_then(|v| v.get::<String>());

                    // Read SpecificObject (AccessPoint path)
                    let ap_path = conn
                        .call_sync(
                            Some("org.freedesktop.NetworkManager"),
                            path,
                            "org.freedesktop.DBus.Properties",
                            "Get",
                            Some(&(
                                "org.freedesktop.NetworkManager.Connection.Active",
                                "SpecificObject",
                            ).to_variant()),
                            None,
                            gio::DBusCallFlags::NONE,
                            -1,
                            gio::Cancellable::NONE,
                        )
                        .ok()
                        .and_then(|res| res.child_value(0).get::<glib::Variant>())
                        .and_then(|v| v.get::<String>());

                    if let Some(ref ap) = ap_path {
                        if ap != "/" && !ap.is_empty() {
                            // Read Strength property on AccessPoint
                            signal = conn
                                .call_sync(
                                    Some("org.freedesktop.NetworkManager"),
                                    ap,
                                    "org.freedesktop.DBus.Properties",
                                    "Get",
                                    Some(&(
                                        "org.freedesktop.NetworkManager.AccessPoint",
                                        "Strength",
                                    ).to_variant()),
                                    None,
                                    gio::DBusCallFlags::NONE,
                                    -1,
                                    gio::Cancellable::NONE,
                                )
                                .ok()
                                .and_then(|res| res.child_value(0).get::<glib::Variant>())
                                .and_then(|v| v.get::<u8>())
                                .map(|s| s as u32);
                        }
                    }
                }
            }

            return NetworkState {
                is_enabled,
                ssid,
                signal,
            };
        }

        // Fallback via CLI if D-Bus system bus is unreachable
        Self::get_state_cli()
    }

    /// Check if Wi-Fi interface radio is enabled
    pub fn is_wifi_enabled() -> bool {
        Self::get_state().is_enabled
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
        Self::get_state().ssid
    }

    /// Get signal strength (0-100%) of active connected Wi-Fi network
    pub fn get_active_signal() -> Option<u32> {
        Self::get_state().signal
    }

    /// Fallback CLI state parser
    fn get_state_cli() -> NetworkState {
        let is_enabled = if let Ok(output) = Command::new("nmcli")
            .env("LC_ALL", "C")
            .args(["radio", "wifi"])
            .output()
        {
            String::from_utf8_lossy(&output.stdout)
                .trim()
                .eq_ignore_ascii_case("enabled")
        } else {
            false
        };

        let mut ssid = None;
        let mut signal = None;

        if is_enabled {
            if let Ok(output) = Command::new("nmcli")
                .env("LC_ALL", "C")
                .args(["-t", "-f", "ACTIVE,SSID,SIGNAL", "dev", "wifi"])
                .output()
            {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    if line.starts_with("yes:") {
                        let parts: Vec<&str> = line["yes:".len()..].split(':').collect();
                        if !parts.is_empty() && !parts[0].is_empty() {
                            ssid = Some(parts[0].trim().to_string());
                        }
                        if parts.len() >= 2 {
                            signal = parts[1].trim().parse::<u32>().ok();
                        }
                        break;
                    }
                }
            }
        }

        NetworkState {
            is_enabled,
            ssid,
            signal,
        }
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

    /// Disconnect from a Wi-Fi connection asynchronously using worker thread
    pub fn disconnect_network(ssid: Option<&str>) {
        let s = ssid.map(|s| s.to_string());
        crate::services::worker::TaskWorker::dispatch(move || {
            if let Some(name) = s {
                let _ = Command::new("nmcli")
                    .env("LC_ALL", "C")
                    .args(["connection", "down", "id", &name])
                    .output();
            } else {
                let _ = Command::new("nmcli")
                    .env("LC_ALL", "C")
                    .args(["device", "disconnect", "wifi"])
                    .output();
            }
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
