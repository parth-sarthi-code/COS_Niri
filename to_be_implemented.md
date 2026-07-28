# COS_Niri Roadmap & Future Implementations (`to_be_implemented.md`)

This document tracks planned architectural enhancements, GPU performance optimizations, native D-Bus services, and UI features for `cos-niri-bar`.

---

## 1. Explicit Hardware-Accelerated GPU Compositing
* **Goal**: Force GTK4's GSK (GTK Scene Kit) to render via high-performance OpenGL/Vulkan GPU pipelines.
* **Implementation Plan**:
  * Set `std::env::set_var("GSK_RENDERER", "ngl")` or `"vulkan"` at the very start of `src/main.rs` before GTK initialization.
  * Bypasses CPU Cairo software fallback rendering entirely for zero-copy GPU texture buffer commits to Niri.

---

## 2. Native BlueZ D-Bus Bluetooth Service
* **Goal**: Replace `bluetoothctl` CLI subprocess polling with a direct D-Bus event listener.
* **Implementation Plan**:
  * Connect directly to `org.bluez` via `zbus` / native D-Bus sockets.
  * Subscribe to `PropertiesChanged` signals for adapter power, connected devices, and battery status.
  * Eliminates process forks and lowers CPU usage to **0.0%**.

---

## 3. Workspace Open App Indicators in `LeftSection`
* **Goal**: Display live app icons (e.g. Chrome, Firefox, Alacritty) inside each active workspace pill.
* **Implementation Plan**:
  * Query `NiriIpcClient::get_windows()` and filter `window.workspace_id == Some(ws.id)`.
  * Render compact app icons dynamically inside workspace pills in `src/components/left.rs`.

---

## 4. Persistent Pinned Apps JSON Configuration (`~/.config/cos-niri/pinned_apps.json`)
* **Goal**: Allow users to customize and persist their pinned dock icons without editing Rust source code.
* **Implementation Plan**:
  * Read/write JSON configuration at `~/.config/cos-niri/pinned_apps.json`.
  * Fallback to default `DEFAULT_PINNED_APPS` if configuration file does not exist.

---

## 5. Fullscreen / Floating Glass App Launcher (ChromeOS / Launchpad Aesthetic)
* **Goal**: Re-introduce a minimalist, hardware-blurred App Launcher popup when the launcher bubble button is clicked.
* **Implementation Plan**:
  * Build with pre-realized Wayland layer-shell surface (`set_visible(false)`).
  * Use sub-millisecond direct POSIX spawner (`execve` + `setsid` + `/dev/null`).
  * Enable Niri layer-rule blur with `background-effect { blur true; xray true; }`.
