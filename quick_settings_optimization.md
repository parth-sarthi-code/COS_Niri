# Quick Settings Panel CPU & I/O Optimization Report

This report documents the performance analysis of the Quick Settings panel components inside `src/components/quick_settings/` and outlines key scopes for optimization.

---

## 1. Identified Performance & Redundancy Issues

### Issue A: Heavy Redundant Syncing of Hidden Sub-pages (High Impact)
*   **Location**: `QuickSettingsPopup::toggle` inside [`popup.rs`](file:///home/predator/COS_Niri/src/components/quick_settings/popup.rs) (Lines 248–249)
*   **What happens**: Whenever the Quick Settings menu is opened, a deferred timeout runs:
    ```rust
    wifi_page_ref.sync_state();
    audio_page_ref.sync_state();
    ```
    This immediately spawns background threads to query PipeWire audio sinks (`pactl` / `AudioService::get_sinks()`) and scans Wi-Fi networks (`nmcli device wifi list` / D-Bus). 
*   **Why it's redundant**: The Wi-Fi and Audio sub-pages are **hidden** by default when opening the main panel (which only shows the `main` stack page). Redrawing hidden list box widgets wastes CPU cycles. Furthermore, when the user actually clicks to open these sub-pages, the click callbacks already call `.sync_state()` right before showing them.

### Issue B: Heavy Unconditional Active Wi-Fi Scanning (High Impact)
*   **Location**: `WifiPage::sync_state` inside [`wifi_page.rs`](file:///home/predator/COS_Niri/src/components/quick_settings/wifi_page.rs) (Line 105)
*   **What happens**: `sync_state` invokes `NetworkService::request_scan()` unconditionally when Wi-Fi is active. This forces the physical network card to perform an active scan for access points.
*   **Why it's heavy**: Toggling the panel frequently forces continuous active network scans, causing transient network latency and high system process CPU usage.

### Issue C: Redundant Label Text setting (Medium Impact)
*   **Location**: `GridSection::async_refresh` inside [`grid.rs`](file:///home/predator/COS_Niri/src/components/quick_settings/grid.rs) (Lines 229–267)
*   **What happens**: Whenever a D-Bus or signal event is received, `async_refresh` updates the Wi-Fi SSID labels, Bluetooth status, and Night Light text unconditionally using `.set_text()`.
*   **Why it's inefficient**: Changing widget text invalidates Pango text layout caches and forces layout reflows on the CPU, even if the text matches exactly (e.g. Wi-Fi remains "Connected").

---

## 2. Recommended Optimization Steps (No Code Implemented)

1.  **Remove Hidden Sub-page Syncs**: Remove `wifi_page_ref.sync_state();` and `audio_page_ref.sync_state();` from the deferred `toggle()` timeout in `popup.rs`. Let them only be synchronized on-demand when the user navigates into those sub-pages.
2.  **Throttle Active Wi-Fi Scans**: Do not request a scan unconditionally. Only trigger `request_scan()` if a scan hasn't occurred in the last 15–30 seconds.
3.  **Implement Value Cache Guarding**: Store the last set status string in internal state or query current widget text first before invoking `.set_text(...)`. Only invoke when text differs.
