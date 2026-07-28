# ChromeOS-Style Niri Bar (`cos-niri-bar`)

A premium, glassmorphic status bar and desktop shell components designed specifically for the Niri Wayland compositor.

---

## Compatibility Note (Fedora Only)
This shell currently officially supports **Fedora** (tested on Fedora 40/41 Workstation). 
*   **D-Bus APIs**: Backend features like Wi-Fi SSID network scanning rely on specific variant signatures returned by Fedora's `org.freedesktop.NetworkManager` implementation. Some features may crash or fail to work on other distributions (like Arch or Ubuntu) due to variations in D-Bus signature schemas.

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
