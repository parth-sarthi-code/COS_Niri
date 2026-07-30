use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::thread;

#[derive(Debug, Clone)]
pub struct BluetoothDevice {
    pub mac: String,
    pub name: String,
    pub is_connected: bool,
}

pub struct BluetoothService;

impl BluetoothService {
    /// Check if Bluetooth adapter is powered ON
    pub fn is_bluetooth_enabled() -> bool {
        if let Ok(output) = Command::new("bluetoothctl")
            .env("LC_ALL", "C")
            .arg("show")
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            return stdout.contains("Powered: yes");
        }
        false
    }

    /// Toggle Bluetooth adapter power ON/OFF asynchronously using worker thread
    pub fn set_bluetooth_enabled(enabled: bool) {
        let arg = if enabled { "on" } else { "off" };
        crate::services::worker::TaskWorker::dispatch(move || {
            let _ = Command::new("bluetoothctl")
                .env("LC_ALL", "C")
                .args(["power", arg])
                .output();
        });
    }

    /// Get list of paired / available Bluetooth devices efficiently (2 forks instead of N+1)
    pub fn get_devices() -> Vec<BluetoothDevice> {
        let mut devices = Vec::new();

        // 1. Get all paired/known devices (1 fork)
        let all_output = Command::new("bluetoothctl")
            .env("LC_ALL", "C")
            .arg("devices")
            .output();

        // 2. Get only connected devices (1 fork) — O(1) vs O(n) per-device info calls
        let mut connected_macs = std::collections::HashSet::new();
        if let Ok(output) = Command::new("bluetoothctl")
            .env("LC_ALL", "C")
            .args(["devices", "Connected"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 && parts[0] == "Device" {
                    connected_macs.insert(parts[1].to_string());
                }
            }
        }

        if let Ok(output) = all_output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 && parts[0] == "Device" {
                    let mac = parts[1].to_string();
                    let name = parts[2..].join(" ");
                    let is_connected = connected_macs.contains(&mac);

                    devices.push(BluetoothDevice {
                        mac,
                        name,
                        is_connected,
                    });
                }
            }
        }

        devices
    }



    /// Connect or disconnect Bluetooth device by MAC address asynchronously using worker thread
    pub fn toggle_device_connection(mac: &str, current_connected: bool) {
        let m = mac.to_string();
        crate::services::worker::TaskWorker::dispatch(move || {
            let action = if current_connected { "disconnect" } else { "connect" };
            let _ = Command::new("bluetoothctl")
                .env("LC_ALL", "C")
                .args([action, &m])
                .output();
        });
    }

    /// Event stream listener via `bluetoothctl` for live state updates
    pub fn listen_events<F>(mut callback: F)
    where
        F: FnMut() + Send + 'static,
    {
        thread::spawn(move || {
            if let Ok(mut child) = Command::new("stdbuf")
                .env("LC_ALL", "C")
                .args(["-oL", "bluetoothctl"])
                .stdout(Stdio::piped())
                .spawn()
            {
                if let Some(stdout) = child.stdout.take() {
                    let reader = BufReader::new(stdout);
                    let mut last_trigger = std::time::Instant::now() - std::time::Duration::from_secs(2);
                    for _line in reader.lines().flatten() {
                        if last_trigger.elapsed() >= std::time::Duration::from_millis(1000) {
                            last_trigger = std::time::Instant::now();
                            callback();
                        }
                    }
                }
            }
        });
    }
}
