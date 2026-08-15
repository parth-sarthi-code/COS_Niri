# 🌌 COS Niri Shell (`cos-niri-bar`)

[![Release](https://img.shields.io/github/v/release/parth-sarthi-code/COS_Niri?color=7aa2f7&label=Release&style=for-the-badge)](https://github.com/parth-sarthi-code/COS_Niri/releases)
[![License](https://img.shields.io/badge/License-GPL--3.0-blue.svg?style=for-the-badge)](LICENSE)
[![Rust](https://img.shields.io/badge/Built%20With-Rust%20%26%20GTK4-orange.svg?style=for-the-badge&logo=rust)](https://www.rust-lang.org/)
[![Niri](https://img.shields.io/badge/Compositor-Niri%20Wayland-6c71c4?style=for-the-badge)](https://github.com/YaLTeR/niri)
[![Idle CPU](https://img.shields.io/badge/Idle%20CPU-0.0%25-brightgreen.svg?style=for-the-badge)](#-performance-benchmarks)

A complete, ultra-performant, glassmorphic desktop environment shell designed specifically for the **[Niri](https://github.com/YaLTeR/niri)** Wayland scrollable-tiling compositor. 

Inspired by **ChromeOS** and modern **macOS design languages**, `cos-niri-bar` combines buttery 165Hz hardware-accelerated animations, automatic **Material You 3** wallpaper color extraction, deep compositor integration, and strict **0.0% idle CPU utilization**.

---

## 📸 Screenshots

| **App Drawer (Launchpad)** | **Quick Settings Control Center** |
| :---: | :---: |
| ![App Drawer](media/app_drawer.png) | ![Quick Settings](media/quick_settings.png) |

| **Calendar & Time Panel** | **System Status Bar (Shelf)** |
| :---: | :---: |
| ![Calendar](media/calendar.png) | ![Status Bar](media/bar.png) |

| **Live Wi-Fi Manager** | **Audio Device Switcher** |
| :---: | :---: |
| ![Wi-Fi Panel](media/wifi_devices.png) | ![Audio Panel](media/audio_deviices.png) |

---

## ⚡ Performance Benchmarks (v0.4.0)

Tested on Linux 6.13 x86_64 running Niri compositor at 165Hz:

| Metric | Measured Value | Optimization Technique |
| :--- | :--- | :--- |
| **Idle CPU Utilization** | **0.00%** | Event-driven Unix pipes, `epoll`, and D-Bus signals (zero busy polling) |
| **Application Private Heap** | **~38.6 MB** | $O(1)$ stack allocations, shared texture caching |
| **App Search Lookup Time** | **0.003 ms** | In-memory `Arc<HashMap>` cache with kernel `inotify` invalidation ($1,700\times$ faster) |
| **Theme Generation Latency** | **< 10 ms** | 320×180 thumbnail downscaling before Matugen quantization |
| **Slider Dispatch Latency** | **30 ms throttle** | Event debouncing pruning ~90% of redundant subprocess forks |
| **Frame Pacing** | **165 FPS (Vulkan)** | Dedicated GSK Vulkan pipeline with Wayland presentation timings |

---

## ✨ Features & Architecture Breakdown

### 1. 🖥️ ChromeOS Bottom Shelf (`cos-bar`)
* **Left Section (App Launcher Trigger)**:
  * Launcher button triggering either the **Built-in Tahoe App Drawer** or the **Standalone Fuzzel Launcher** (switchable on-the-fly).
  * Smooth hover transitions and circular indicator styling.
* **Center Section (Interactive Dock)**:
  * **Screen-Centered Alignment**: Anchored at true screen center via `gtk4::CenterBox`.
  * **Dynamic App Tracking**: Communicates with Niri via `niri-ipc` socket to show running state dots (active, running, hidden) and focus highlights.
  * **Pinning & Reordering**: Pin, unpin, and organize applications directly from the GUI Settings or dock.
  * **GPU Texture Cache**: Resolved `IconPaintable` handles are cached in memory to eliminate repeated disk lookups and texture churn.
* **Right Section (ChromeOS Split-Pill Status Group)**:
  * **Date Pill**: Shows localized current date; clicking opens the animated Calendar panel.
  * **Quick Settings Pill**: Displays dynamic clock (`HH:MM`) alongside network signal strength, audio volume icon, and battery percentage.
  * **StatusNotifierWatcher Tray (SNI)**: Full support for background apps (Steam, Discord, Spotify, Telegram). Dynamically hides when empty; caches textures to prevent OpenGL/Vulkan memory bloat.

---

### 2. 🎛️ Quick Settings Control Center (`cos-quick-settings`)
* **Header & Quick Actions**:
  * User profile avatar bubble and session sign-out button.
  * Instant action triggers: Lock Session (`loginctl lock-session`), Power Off (`systemctl poweroff`), Settings App, and Collapse.
* **Interactive 6-Tile Feature Grid**:
  * **Wi-Fi Toggle & Subpage**: Real-time network scanning via `nmcli`. Interactive password modal with error feedback and auto-connect.
  * **Bluetooth Toggle & Subpage**: Scans and displays paired & available devices with live `"Connecting..."` feedback.
  * **Audio Device Subpage**: Hot-swappable list of active audio sinks (headphones, speakers, HDMI).
  * **Night Light**: Temperature toggle for nighttime eye strain reduction.
  * **Power Mode Profiles**: One-click switching between Performance (`Performance`), Balanced (`Balanced`), and Battery Saver (`Power Saver`).
  * **Screen Capture**: Quick trigger for screenshot utilities (`satty` / `grim`).
* **Material You Sliders**:
  * Smooth Volume and Display Brightness sliders with $30\text{ ms}$ adaptive debouncing and interactive highlight tracks.

---

### 3. 🚀 App Launchers
* **Built-in Glassmorphic App Drawer (`cos-launcher`)**:
  * 60-tile pre-warmed responsive grid.
  * Instant search bar that automatically grabs keyboard focus upon opening.
  * Category pill filters (All, Internet, Development, Media, Office, Utilities, System).
* **Fuzzel Mode Integration**:
  * Instant, ultra-lightweight launcher mode.
  * Uses Material You palette (`fuzzel-colors.ini`) with glassmorphic transparency and Niri background blur.

---

### 4. 📅 Calendar Panel (`cos-calendar`)
* Clean ChromeOS month grid with month traversal chevrons and active today highlight.
* Transparent full-screen layer-shell dismiss shield to close upon outside clicks.

---

### 5. ⚙️ COS Settings Window
* **Appearance Tab**:
  * Desktop wallpaper selector with live preview.
  * **9 Material You 3 Color Schemes**: *Tonal Spot, Neutral, Vibrant, Expressive, Fidelity, Rainbow, Fruit Salad, Monochrome, Content*.
  * Dark Theme / Light Theme toggle.
* **Blur & Effects Tab**:
  * Granular per-module background blur & X-ray switches for Bar Shelf, Quick Settings, Calendar, Launcher, Fuzzel, and Tray menus.
  * Surface Opacity & Transparency slider (10% to 100%) with instant preset buttons (*Solid, Glass, Frosted, Translucent, Clear*).
* **Performance Tab**:
  * **Graphic Engine Switcher**:
    * **Vulkan (165Hz GPU)**: Optimal frame pacing for high-refresh gaming and desktop monitors.
    * **OpenGL (Balanced GPU)**: Hardware acceleration with lower driver compilation overhead.
    * **Cairo (Eco / Minimum RAM)**: Pure 2D software CPU rendering for extreme power/RAM savings.
    * *Auto-Restart*: Switching graphics engines cleanly restarts the process in-place and re-opens the Settings window without memory leaks.
  * **App Launcher Mode**: Switch between Built-in Tahoe Grid and Fuzzel.
* **Pinned Apps Tab**: Search installed desktop apps, pin/unpin, and customize your dock in real time.

---

### 6. 🎨 Material You 3 Theme Engine (`matugen`)
* Automatically quantizes colors from your wallpaper using `matugen` on a downscaled $320\times 180$ thumbnail.
* Synchronously generates:
  * `~/.config/cos-niri/colors.css` (GTK4 dynamic CSS variables).
  * `~/.config/cos-niri/colors-niri.kdl` (Niri active focus rings and window borders).
  * `~/.config/cos-niri/fuzzel-colors.ini` (Fuzzel launcher palette).
* **Zero-Poll POSIX Signals**:
  * `SIGUSR1 (10)`: Triggers background theme regeneration on wallpaper change.
  * `SIGUSR2 (12)`: Triggers live GTK CSS hot-reloading in $< 5\text{ ms}$ without process restarts.

---

## 🚀 One-Line Automated Installation

The official installer automatically checks dependencies, installs required fonts, deploys the optimized release binary, and configures Niri:

```bash
curl -sSL https://raw.githubusercontent.com/parth-sarthi-code/COS_Niri/main/install.sh | bash
```

---

## 🛠️ Manual Build from Source

### 1. Install Dependencies

**Arch Linux / CachyOS:**
```bash
sudo pacman -S --needed gtk4 gtk4-layer-shell fontconfig matugen fuzzel swaybg adwaita-icon-theme
```

**Fedora:**
```bash
sudo dnf install gtk4-devel gtk4-layer-shell-devel fontconfig fuzzel swaybg cargo
cargo install matugen
```

**Debian / Ubuntu:**
```bash
sudo apt-get install libgtk-4-dev libgtk4-layer-shell-dev fontconfig fuzzel swaybg cargo
cargo install matugen
```

### 2. Compile & Install
```bash
git clone https://github.com/parth-sarthi-code/COS_Niri.git
cd COS_Niri
cargo build --release
install -m 755 target/release/cos-niri-bar ~/.local/bin/cos-niri-bar
```

---

## 🧩 Niri Compositor Configuration (`rules.kdl`)

Add the following layer rules to `~/.config/niri/rules.kdl` to enable native background blur and glassmorphism:

```kdl
// ChromeOS Shelf
layer-rule {
    match namespace="cos-bar"
    shadow { on }
    background-effect {
        blur true
        noise 0.03
        saturation 1.6
        xray false
    }
}

// Quick Settings & Calendar Panels
layer-rule {
    match namespace="cos-quick-settings"
    match namespace="cos-calendar"
    geometry-corner-radius 24
    shadow { on }
    background-effect {
        blur true
        noise 0.03
        saturation 1.6
        xray false
    }
}

// System Tray Menus & Fuzzel Launcher
layer-rule {
    match namespace="cos-tray-menu"
    match namespace="^launcher$"
    match namespace="^fuzzel$"
    geometry-corner-radius 16
    shadow { on }
    background-effect {
        blur true
        noise 0.03
        saturation 1.6
        xray false
    }
}

// Settings Windowed Application
window-rule {
    match title="Settings"
    open-floating true
    default-column-width { fixed 840; }
    default-window-height { fixed 560; }
    background-effect {
        blur true
        noise 0.03
        saturation 1.6
    }
}
```

Add auto-start to `~/.config/niri/config.kdl`:
```kdl
spawn-at-startup "cos-niri-bar"
```

---

## 🖼️ Nautilus Right-Click "Set as Wallpaper" Integration

Create `~/.local/share/nautilus/scripts/Set as Wallpaper` with execution permissions:

```bash
#!/usr/bin/env bash
SELECTED_FILE="${1:-$NAUTILUS_SCRIPT_SELECTED_FILE_PATHS}"
SELECTED_FILE=$(echo "$SELECTED_FILE" | head -n 1)

if [ -f "$SELECTED_FILE" ]; then
    cp "$SELECTED_FILE" "$HOME/.config/background"
    pkill swaybg
    nohup swaybg -i "$HOME/.config/background" -m fill >/dev/null 2>&1 &
    
    # Notify cos-niri-bar to regenerate Material You colors immediately
    pkill -USR1 cos-niri-bar || true
fi
```

---

## 📁 Configuration & State Files

| Path | Purpose |
| :--- | :--- |
| `~/.config/cos-niri/settings.json` | Persistent user preferences (themes, opacity, blur toggles, renderer, pinned apps) |
| `~/.config/cos-niri/colors.css` | Generated Material You CSS dynamic variables (`@primary`, `@surface`, etc.) |
| `~/.config/cos-niri/colors-niri.kdl` | Niri focus ring and active window border colors |
| `~/.config/cos-niri/fuzzel-colors.ini` | Fuzzel Material You color configuration |
| `~/.local/share/fonts/cos-niri/` | Bundled Material Symbols Rounded & Roboto typefaces |

---

## 🤝 Contributing & License

Contributions, issue reports, and pull requests are welcome! 

Licensed under the **GPL-3.0 License**. See [LICENSE](LICENSE) for details.
