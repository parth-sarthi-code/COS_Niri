# Implemented Features

This document provides a comprehensive list of features, performance improvements, and layout styles implemented in `cos-niri-bar`.

---

## 1. macOS Launchpad App Drawer (`cos-launcher`)
*   **True Fullscreen Overlay**: Window anchors span all four screen edges, bypassing Niri's exclusive layout margins.
*   **Bottom Gap Elimination**: Configured with a `-80px` bottom margin to extend the window surface completely below the bar shelf boundary.
*   **8-Column Grid**: Tightly structured FlowBox layout (`max_children_per_line=8`, `min_children_per_line=8`) with compact spacing (`18px` row spacing, `16px` column spacing) and 48px icons in 72x72 bubbles. This optimizes density to fit more applications on one screen and reduce scrolling.
*   **Dynamic Search Focus**: Automatically sets keyboard focus to `OnDemand` when toggled open and grabs input focus inside the search box, allowing search queries to be typed immediately. Releases keyboard mode to `None` on close.
*   **Escape Key Dismissal**: Configured with an `EventControllerKey` in `Capture` phase to intercept the `Escape` key and slide the window closed immediately, even when text is active in the search entry.
*   **Helper Filtering**: Ignores helper configuration binaries and hidden setting options (filtering desktop entries with `NoDisplay=true`).

---

## 2. Quick Settings Panel (`cos-quick-settings`)
*   **Variant D-Bus Parse Correction**: Corrected SSID variant-in-variant parsing under NetworkManager to prevent D-Bus variant extraction panics.
*   **D-Bus Wi-Fi Scan CPU Optimizations**: Replaced blocking loops with non-blocking polling intervals to yield thread cycles to GLib.
*   **Merged Subprocesses**: Combined redundant CLI tool execution forks (e.g. `wpctl get-volume` and `is_muted`) into single-spawn queries.
*   **O(1) Bluetooth Connection Checking**: Batch filters connections in a single shell command rather than looping info checks.

---

## 3. Calendar Panel (`cos-calendar`)
*   **Dynamic Date Navigation**: Allows moving months forward/backward and highlighting the current day.
*   **Animated Dismissals**: Synchronized sliding animations matching display refresh rates (up to 165Hz).
*   **Non-Stacking Clock**: Restructured the clock update loop to prevent timer stacking leaks.
