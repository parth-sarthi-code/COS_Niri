use std::process::Command;
use std::thread;

#[derive(Debug, Clone)]
pub struct BluetoothDevice {
    pub mac: String,
    pub name: String,
    pub is_connected: bool,
    pub is_paired: bool,
}

pub struct BluetoothService;

impl BluetoothService {
    /// Check if Bluetooth adapter is powered ON
    pub fn is_bluetooth_enabled() -> bool {
        if let Ok(output) = Command::new("bluetoothctl").arg("show").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            return stdout.contains("Powered: yes");
        }
        false
    }

    /// Toggle Bluetooth adapter power ON/OFF
    pub fn set_bluetooth_enabled(enabled: bool) {
        let arg = if enabled { "on" } else { "off" };
        let _ = Command::new("bluetoothctl").args(["power", arg]).output();
    }

    /// Get list of paired / available Bluetooth devices
    pub fn get_devices() -> Vec<BluetoothDevice> {
        let mut devices = Vec::new();

        if let Ok(output) = Command::new("bluetoothctl").arg("devices").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                // Line format: "Device 00:11:22:33:44:55 Device Name"
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 && parts[0] == "Device" {
                    let mac = parts[1].to_string();
                    let name = parts[2..].join(" ");
                    let (is_connected, is_paired) = Self::get_device_status(&mac);

                    devices.push(BluetoothDevice {
                        mac,
                        name,
                        is_connected,
                        is_paired,
                    });
                }
            }
        }

        devices
    }

    fn get_device_status(mac: &str) -> (bool, bool) {
        if let Ok(output) = Command::new("bluetoothctl").args(["info", mac]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let connected = stdout.contains("Connected: yes");
            let paired = stdout.contains("Paired: yes");
            return (connected, paired);
        }
        (false, false)
    }

    /// Connect or disconnect Bluetooth device by MAC address
    pub fn toggle_device_connection(mac: &str, current_connected: bool) {
        let m = mac.to_string();
        thread::spawn(move || {
            let action = if current_connected { "disconnect" } else { "connect" };
            let _ = Command::new("bluetoothctl").args([action, &m]).output();
        });
    }
}
