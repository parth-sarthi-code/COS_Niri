use std::process::Command;

#[derive(Debug, Clone)]
pub struct WifiNetwork {
    pub ssid: String,
    pub signal: u32,
    pub is_connected: bool,
    pub is_known: bool,
}

#[derive(Debug, Clone, Default)]
pub struct NetworkState {
    pub is_enabled: bool,
    pub ssid: Option<String>,
    pub signal: Option<u32>,
}

pub struct NetworkService;

impl NetworkService {
    /// Get list of saved Wi-Fi connection SSIDs
    pub fn get_known_wifi_ssids() -> std::collections::HashSet<String> {
        let mut ssids = std::collections::HashSet::new();
        if let Ok(output) = Command::new("nmcli")
            .args(["-t", "-f", "NAME,TYPE", "connection", "show"])
            .output()
        {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                for line in text.lines() {
                    let parts: Vec<&str> = line.split(':').collect();
                    if parts.len() >= 2 && parts[1] == "802-11-wireless" {
                        ssids.insert(parts[0].to_string());
                    }
                }
            }
        }
        ssids
    }

    /// Get unified NetworkManager state via nmcli
    pub fn get_state() -> NetworkState {
        let is_enabled = Self::is_wifi_enabled();
        if !is_enabled {
            return NetworkState {
                is_enabled: false,
                ssid: None,
                signal: None,
            };
        }

        let mut ssid = None;
        let mut signal = None;

        // Query active wifi connection via nmcli -t -f ACTIVE,SSID,SIGNAL dev wifi list
        if let Ok(output) = Command::new("nmcli")
            .args(["-t", "-f", "ACTIVE,SSID,SIGNAL", "dev", "wifi", "list", "--rescan", "no"])
            .output()
        {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                for line in text.lines() {
                    let fields = parse_terse_line(line);
                    if fields.len() >= 3 {
                        let active = &fields[0];
                        let s_name = &fields[1];
                        let s_sig = fields[2].parse::<u32>().unwrap_or(0);

                        if (active == "yes" || active == "*") && !s_name.is_empty() {
                            ssid = Some(s_name.clone());
                            signal = Some(s_sig);
                            break;
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
    }

    /// Check if Wi-Fi interface is enabled via nmcli
    pub fn is_wifi_enabled() -> bool {
        if let Ok(output) = Command::new("nmcli").args(["radio", "wifi"]).output() {
            if output.status.success() {
                let status = String::from_utf8_lossy(&output.stdout).trim().to_lowercase();
                return status == "enabled";
            }
        }
        false
    }

    /// Toggle Wi-Fi state via nmcli
    pub fn set_wifi_enabled(enable: bool) {
        let arg = if enable { "on" } else { "off" };
        crate::services::worker::TaskWorker::dispatch(move || {
            let _ = Command::new("nmcli").args(["radio", "wifi", arg]).output();
            if enable {
                let _ = Command::new("nmcli").args(["dev", "wifi", "rescan"]).output();
            }
        });
    }

    /// Trigger background Wi-Fi scan via nmcli
    pub fn request_scan() {
        crate::services::worker::TaskWorker::dispatch(move || {
            let _ = Command::new("nmcli").args(["dev", "wifi", "rescan"]).output();
        });
    }

    /// Get active SSID
    pub fn get_active_ssid() -> Option<String> {
        Self::get_state().ssid
    }

    /// Get active signal percentage
    pub fn get_active_signal() -> Option<u32> {
        Self::get_state().signal
    }

    pub fn scan_networks() -> Vec<WifiNetwork> {
        Self::get_scanned_networks()
    }

    /// Scan Wi-Fi access points via nmcli (DSA Optimized: O(N) Hash Deduplication + O(N log N) Priority Sorting)
    pub fn get_scanned_networks() -> Vec<WifiNetwork> {
        if !Self::is_wifi_enabled() {
            return Vec::new();
        }

        let known_ssids = Self::get_known_wifi_ssids();
        let mut map = std::collections::HashMap::<String, WifiNetwork>::new();

        if let Ok(output) = Command::new("nmcli")
            .args(["-t", "-f", "ACTIVE,SSID,SIGNAL", "dev", "wifi", "list", "--rescan", "no"])
            .output()
        {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                for line in text.lines() {
                    let fields = parse_terse_line(line);
                    if fields.len() >= 3 {
                        let active = &fields[0];
                        let ssid_name = fields[1].trim();
                        let signal_val = fields[2].parse::<u32>().unwrap_or(0);

                        if ssid_name.is_empty() {
                            continue;
                        }

                        let is_connected = active == "yes" || active == "*";
                        let is_known = known_ssids.contains(ssid_name);

                        map.entry(ssid_name.to_string())
                            .and_modify(|existing| {
                                existing.is_connected = existing.is_connected || is_connected;
                                existing.signal = existing.signal.max(signal_val);
                            })
                            .or_insert(WifiNetwork {
                                ssid: ssid_name.to_string(),
                                signal: signal_val,
                                is_connected,
                                is_known,
                            });
                    }
                }
            }
        }

        let mut list: Vec<WifiNetwork> = map.into_values().collect();
        // Priority sort: Connected network ranks #1, followed by descending signal strength
        list.sort_by(|a, b| {
            b.is_connected
                .cmp(&a.is_connected)
                .then_with(|| b.signal.cmp(&a.signal))
        });
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
            cmd.args(["dev", "wifi", "connect", &s]);
            if let Some(ref pass) = p {
                if !pass.is_empty() {
                    cmd.args(["password", pass]);
                }
            }
            let _ = cmd.output();
        });
    }

    pub fn disconnect_network(ssid: Option<&str>) {
        let s = ssid.map(|s| s.to_string());
        crate::services::worker::TaskWorker::dispatch(move || {
            if let Some(ref ssid_name) = s {
                let _ = Command::new("nmcli")
                    .args(["connection", "down", "id", ssid_name])
                    .output();
            } else {
                if let Ok(output) = Command::new("nmcli").args(["-t", "-f", "DEVICE,TYPE", "dev"]).output() {
                    if output.status.success() {
                        let text = String::from_utf8_lossy(&output.stdout);
                        for line in text.lines() {
                            let parts: Vec<&str> = line.split(':').collect();
                            if parts.len() >= 2 && parts[1] == "wifi" {
                                let dev = parts[0];
                                let _ = Command::new("nmcli").args(["dev", "disconnect", dev]).output();
                                break;
                            }
                        }
                    }
                }
            }
        });
    }

    /// D-Bus event stream listener for NetworkManager signals (keeps live UI in sync)
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

                let loop_ctx = glib::MainContext::new();
                let main_loop = glib::MainLoop::new(Some(&loop_ctx), false);
                let _ = loop_ctx.with_thread_default(|| {
                    main_loop.run();
                });
            }
        });
    }
}

/// Helper to parse colon-separated nmcli -t lines handling escaped colons (`\:`)
fn parse_terse_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&next) = chars.peek() {
                if next == ':' || next == '\\' {
                    current.push(next);
                    chars.next();
                    continue;
                }
            }
            current.push(c);
        } else if c == ':' {
            fields.push(current);
            current = String::new();
        } else {
            current.push(c);
        }
    }
    fields.push(current);
    fields
}
