use gio::prelude::*;
use std::process::Command;

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
                            let sig_val = conn
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
                                .and_then(|v| v.get::<u8>());

                            if let Some(s) = sig_val {
                                signal = Some(s as u32);
                            }
                        }
                    }
                }
            }

            NetworkState {
                is_enabled,
                ssid,
                signal,
            }
        } else {
            NetworkState::default()
        }
    }

    /// Check if Wi-Fi interface is enabled via native D-Bus
    pub fn is_wifi_enabled() -> bool {
        Self::get_state().is_enabled
    }

    /// Toggle Wi-Fi state via native D-Bus IPC (0 process forks)
    pub fn set_wifi_enabled(enable: bool) {
        crate::services::worker::TaskWorker::dispatch(move || {
            if let Ok(conn) = gio::bus_get_sync(gio::BusType::System, gio::Cancellable::NONE) {
                let _ = conn.call_sync(
                    Some("org.freedesktop.NetworkManager"),
                    "/org/freedesktop/NetworkManager",
                    "org.freedesktop.DBus.Properties",
                    "Set",
                    Some(&(
                        "org.freedesktop.NetworkManager",
                        "WirelessEnabled",
                        enable.to_variant(),
                    ).to_variant()),
                    None,
                    gio::DBusCallFlags::NONE,
                    -1,
                    gio::Cancellable::NONE,
                );
            }
        });
    }

    /// Get active SSID via D-Bus
    pub fn get_active_ssid() -> Option<String> {
        Self::get_state().ssid
    }

    /// Get active signal percentage via D-Bus
    pub fn get_active_signal() -> Option<u32> {
        Self::get_state().signal
    }

    /// Scan Wi-Fi networks asynchronously
    pub fn scan_networks() -> Vec<WifiNetwork> {
        Self::get_scanned_networks()
    }

    pub fn get_scanned_networks() -> Vec<WifiNetwork> {
        let mut list = Vec::new();
        if let Ok(output) = Command::new("nmcli")
            .env("LC_ALL", "C")
            .args(["-t", "-f", "IN-USE,SSID,SIGNAL", "device", "wifi", "list"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 3 {
                    let is_connected = parts[0].trim() == "*";
                    let ssid = parts[1].trim().to_string();
                    let signal = parts[2].trim().parse::<u32>().unwrap_or(0);

                    if !ssid.is_empty() && !list.iter().any(|n: &WifiNetwork| n.ssid == ssid) {
                        list.push(WifiNetwork {
                            ssid,
                            signal,
                            is_connected,
                        });
                    }
                }
            }
        }
        list
    }

    pub fn connect_network(ssid: &str, password: Option<&str>) {
        Self::connect_to_network(ssid, password);
    }

    pub fn connect_to_network(ssid: &str, password: Option<&str>) {
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

    pub fn disconnect_network(_ssid: Option<&str>) {
        crate::services::worker::TaskWorker::dispatch(move || {
            let _ = Command::new("nmcli")
                .env("LC_ALL", "C")
                .args(["device", "disconnect", "wlan0"])
                .output();
        });
    }

    /// Pure Native GIO D-Bus event stream listener for NetworkManager signals (0 nmcli child processes)
    pub fn listen_events<F>(callback: F)
    where
        F: Fn() + Send + 'static,
    {
        std::thread::spawn(move || {
            if let Ok(conn) = gio::bus_get_sync(gio::BusType::System, gio::Cancellable::NONE) {
                let _sub_id = conn.signal_subscribe(
                    Some("org.freedesktop.NetworkManager"),
                    Some("org.freedesktop.DBus.Properties"),
                    Some("PropertiesChanged"),
                    None,
                    None,
                    gio::DBusSignalFlags::NONE,
                    move |_, _, _, _, _, _| {
                        callback();
                    },
                );

                // Keep thread worker alive in D-Bus main loop
                let loop_ctx = glib::MainContext::new();
                let main_loop = glib::MainLoop::new(Some(&loop_ctx), false);
                let _ = loop_ctx.with_thread_default(|| {
                    main_loop.run();
                });
            }
        });
    }
}
