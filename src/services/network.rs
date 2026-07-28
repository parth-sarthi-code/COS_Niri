use gio::prelude::*;

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

                if enable {
                    Self::request_scan_internal(&conn);
                }
            }
        });
    }

    /// Trigger background Wi-Fi scan over D-Bus
    pub fn request_scan() {
        crate::services::worker::TaskWorker::dispatch(move || {
            if let Ok(conn) = gio::bus_get_sync(gio::BusType::System, gio::Cancellable::NONE) {
                Self::request_scan_internal(&conn);
            }
        });
    }

    fn request_scan_internal(conn: &gio::DBusConnection) {
        if let Some(wifi_dev) = Self::get_wifi_device_path(conn) {
            let empty_dict: std::collections::HashMap<String, glib::Variant> = std::collections::HashMap::new();
            let _ = conn.call_sync(
                Some("org.freedesktop.NetworkManager"),
                &wifi_dev,
                "org.freedesktop.NetworkManager.Device.Wireless",
                "RequestScan",
                Some(&(empty_dict,).to_variant()),
                None,
                gio::DBusCallFlags::NONE,
                -1,
                gio::Cancellable::NONE,
            );
        }
    }

    /// Find Wi-Fi device object path (DeviceType == 2)
    fn get_wifi_device_path(conn: &gio::DBusConnection) -> Option<String> {
        let res = conn
            .call_sync(
                Some("org.freedesktop.NetworkManager"),
                "/org/freedesktop/NetworkManager",
                "org.freedesktop.NetworkManager",
                "GetDevices",
                None,
                None,
                gio::DBusCallFlags::NONE,
                -1,
                gio::Cancellable::NONE,
            )
            .ok()?;

        let var = res.child_value(0);
        let mut dev_paths = Vec::new();
        for i in 0..var.n_children() {
            let elem = var.child_value(i);
            if let Some(s) = elem.str() {
                dev_paths.push(s.to_string());
            }
        }

        for path in dev_paths {
            let dev_type = conn
                .call_sync(
                    Some("org.freedesktop.NetworkManager"),
                    &path,
                    "org.freedesktop.DBus.Properties",
                    "Get",
                    Some(&(
                        "org.freedesktop.NetworkManager.Device",
                        "DeviceType",
                    ).to_variant()),
                    None,
                    gio::DBusCallFlags::NONE,
                    -1,
                    gio::Cancellable::NONE,
                )
                .ok()
                .and_then(|res| res.child_value(0).get::<glib::Variant>())
                .and_then(|v| v.get::<u32>());

            if dev_type == Some(2) {
                return Some(path);
            }
        }
        None
    }

    /// Get active SSID via D-Bus
    pub fn get_active_ssid() -> Option<String> {
        Self::get_state().ssid
    }

    /// Get active signal percentage via D-Bus
    pub fn get_active_signal() -> Option<u32> {
        Self::get_state().signal
    }

    pub fn scan_networks() -> Vec<WifiNetwork> {
        Self::get_scanned_networks()
    }

    /// Scan Wi-Fi access points 100% natively via System D-Bus IPC (0 process forks)
    pub fn get_scanned_networks() -> Vec<WifiNetwork> {
        let mut list = Vec::new();
        let state = Self::get_state();
        if !state.is_enabled {
            return list;
        }

        if let Ok(conn) = gio::bus_get_sync(gio::BusType::System, gio::Cancellable::NONE) {
            if let Some(wifi_dev) = Self::get_wifi_device_path(&conn) {
                // Call GetAllAccessPoints to fetch all cached and live access points
                let ap_paths_val = conn
                    .call_sync(
                        Some("org.freedesktop.NetworkManager"),
                        &wifi_dev,
                        "org.freedesktop.NetworkManager.Device.Wireless",
                        "GetAllAccessPoints",
                        None,
                        None,
                        gio::DBusCallFlags::NONE,
                        -1,
                        gio::Cancellable::NONE,
                    )
                    .ok()
                    .map(|res| res.child_value(0));

                let mut ap_paths = Vec::new();
                if let Some(ap_var) = ap_paths_val {
                    for i in 0..ap_var.n_children() {
                        let elem = ap_var.child_value(i);
                        if let Some(s) = elem.str() {
                            ap_paths.push(s.to_string());
                        }
                    }
                }

                let active_ssid = state.ssid.unwrap_or_default();

                for ap_path in ap_paths {
                    // Fetch all AccessPoint properties in a single D-Bus round-trip
                    let all_props = conn
                        .call_sync(
                            Some("org.freedesktop.NetworkManager"),
                            &ap_path,
                            "org.freedesktop.DBus.Properties",
                            "GetAll",
                            Some(&("org.freedesktop.NetworkManager.AccessPoint",).to_variant()),
                            None,
                            gio::DBusCallFlags::NONE,
                            -1,
                            gio::Cancellable::NONE,
                        )
                        .ok()
                        .map(|res| res.child_value(0));

                    let props_var = match all_props {
                        Some(v) => v,
                        None => continue,
                    };

                    // Parse SSID from `ay` byte array and Strength from `y`
                    // props_var is a{sv}: dict of (string, variant)
                    let mut ssid_bytes = Vec::new();
                    let mut strength: u32 = 0;

                    for i in 0..props_var.n_children() {
                        let entry = props_var.child_value(i);
                        let key = match entry.child_value(0).str() {
                            Some(k) => k.to_string(),
                            None => continue,
                        };
                        // child_value(1) is the `v` (boxed Variant) — unwrap one level
                        let boxed = entry.child_value(1);
                        let inner = if boxed.type_().as_str() == "v" {
                            boxed.child_value(0)
                        } else {
                            boxed
                        };

                        match key.as_str() {
                            "Ssid" => {
                                for k in 0..inner.n_children() {
                                    if let Some(b) = inner.child_value(k).get::<u8>() {
                                        ssid_bytes.push(b);
                                    }
                                }
                            }
                            "Strength" => {
                                strength = inner.get::<u8>().unwrap_or(0) as u32;
                            }
                            _ => {}
                        }
                    }

                    let ssid_str = String::from_utf8(ssid_bytes).unwrap_or_default();
                    if ssid_str.is_empty() {
                        continue;
                    }

                    let is_connected = !active_ssid.is_empty() && ssid_str == active_ssid;

                    if !list.iter().any(|n: &WifiNetwork| n.ssid == ssid_str) {
                        list.push(WifiNetwork {
                            ssid: ssid_str,
                            signal: strength,
                            is_connected,
                        });
                    }
                }
            }
        }

        list.sort_by(|a, b| b.signal.cmp(&a.signal));
        list
    }

    pub fn connect_network(ssid: &str, password: Option<&str>) {
        Self::connect_to_network(ssid, password);
    }

    pub fn connect_to_network(ssid: &str, password: Option<&str>) {
        let s = ssid.to_string();
        let p = password.map(|pass| pass.to_string());
        crate::services::worker::TaskWorker::dispatch(move || {
            if let Ok(conn) = gio::bus_get_sync(gio::BusType::System, gio::Cancellable::NONE) {
                if let Some(wifi_dev) = Self::get_wifi_device_path(&conn) {
                    let mut connection_dict = std::collections::HashMap::new();

                    let mut s_wireless = std::collections::HashMap::new();
                    s_wireless.insert("ssid".to_string(), s.as_bytes().to_variant());
                    connection_dict.insert("802-11-wireless".to_string(), s_wireless);

                    if let Some(pass) = p {
                        let mut security = std::collections::HashMap::new();
                        security.insert("key-mgmt".to_string(), "wpa-psk".to_variant());
                        security.insert("psk".to_string(), pass.to_variant());
                        connection_dict.insert("802-11-wireless-security".to_string(), security);
                    }

                    let _ = conn.call_sync(
                        Some("org.freedesktop.NetworkManager"),
                        "/org/freedesktop/NetworkManager",
                        "org.freedesktop.NetworkManager",
                        "AddAndActivateConnection",
                        Some(&(
                            connection_dict,
                            wifi_dev,
                            "/",
                        ).to_variant()),
                        None,
                        gio::DBusCallFlags::NONE,
                        -1,
                        gio::Cancellable::NONE,
                    );
                }
            }
        });
    }

    pub fn disconnect_network(_ssid: Option<&str>) {
        crate::services::worker::TaskWorker::dispatch(move || {
            if let Ok(conn) = gio::bus_get_sync(gio::BusType::System, gio::Cancellable::NONE) {
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

                if let Some(path) = primary_path {
                    if path != "/" && !path.is_empty() {
                        let _ = conn.call_sync(
                            Some("org.freedesktop.NetworkManager"),
                            "/org/freedesktop/NetworkManager",
                            "org.freedesktop.NetworkManager",
                            "DeactivateConnection",
                            Some(&(path,).to_variant()),
                            None,
                            gio::DBusCallFlags::NONE,
                            -1,
                            gio::Cancellable::NONE,
                        );
                    }
                }
            }
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
