# ChromeOS-Style Niri Bar (`cos-niri-bar`)

A premium, glassmorphic status bar and desktop shell components designed specifically for the Niri Wayland compositor.

---

## Compatibility Note (Fedora Only)
This shell currently officially supports **Fedora** (tested on Fedora 44 Workstation). 
*   **D-Bus APIs**: Backend features like Wi-Fi SSID network scanning rely on specific variant signatures returned by Fedora's `org.freedesktop.NetworkManager` implementation. Some features may crash or fail to work on other distributions (like Arch or Ubuntu) due to variations in D-Bus signature schemas.

---

## Resource Consumption (Fedora 44)
Measured metrics of the compiled release bar at idle:
*   **CPU (Idle)**: **0.0%**
*   **Resident RAM (RSS)**: **~144 MB** (148,060 KB)
*   **Shared Memory (SHR)**: **~74 MB** (76,480 KB)

---

## Installation & Build Instructions

### 1. Prerequisites
Ensure you have the required GTK4 libraries and Layer-Shell development packages installed:
```bash
sudo dnf install gtk4-devel gtk4-layer-shell-devel
```

### 2. Build Release Binary
Compile the optimized release target:
```bash
cargo build --release
```

### 3. Run Bar on Startup
Copy the binary to your path and configure Niri to spawn it automatically in `/home/predator/.config/niri/config.kdl` (or `rules.kdl`):
```kdl
spawn-at-startup "cos-niri-bar"
```

---

## Niri Compositor Window Rules (`rules.kdl`)
Add the following window rules to your Niri configuration to enable background blur and premium glassmorphic styling:

```kdl
// ChromeOS Bar — blur + shadow for glass shelf effect
layer-rule {
    match namespace="cos-bar"
    shadow {
        on
    }
    background-effect {
        blur true
        xray false
        noise 0.03
        saturation 1.6
    }
}

// ChromeOS Calendar Popup — blur + shadow + rounded corners
layer-rule {
    match namespace="cos-calendar"
    geometry-corner-radius 24
    shadow {
        on
    }
    background-effect {
        blur true
        xray false
        noise 0.03
        saturation 1.6
    }
}

// ChromeOS Quick Settings Popup — blur + shadow + rounded corners
layer-rule {
    match namespace="cos-quick-settings"
    geometry-corner-radius 24
    shadow {
        on
    }
    background-effect {
        blur true
        xray false
        noise 0.03
        saturation 1.6
    }
}

// ChromeOS App Launcher Popup (Fullscreen App Drawer)
layer-rule {
    match namespace="cos-launcher"
    geometry-corner-radius 0
    shadow {
        off
    }
    background-effect {
        blur true
        noise 0.03
        saturation 1.6
    }
}
```

---

## Screenshots
Here are screenshots of the components running under Fedora 44 + Niri:

| macOS Launchpad App Drawer | Quick Settings Control Center |
| :---: | :---: |
| ![App Drawer](media/app_drawer.png) | ![Quick Settings](media/quick_settings.png) |

| Calendar Panel | Status Bar |
| :---: | :---: |
| ![Calendar](media/calendar.png) | ![Status Bar](media/bar.png) |

| WiFi Connections | Audio Output Select |
| :---: | :---: |
| ![WiFi Panel](media/wifi_devices.png) | ![Audio Panel](media/audio_deviices.png) |
