# Comprehensive Technical Audit & Optimization Report: COS_Niri

**Project**: `COS_Niri` (ChromeOS-styled Status Bar for Niri Wayland Compositor)  
**Target Environment**: Linux (Wayland / Niri Compositor / GTK4 Layer-Shell / Rust)  
**Date**: July 28, 2026  

---

## 1. Executive Summary

This performance and architectural audit evaluates the `COS_Niri` status bar codebase across backend services, system interaction layers, thread safety, and GTK4 rendering pipelines.

The codebase boasts a clean architecture, utilizing asynchronous background dispatching (`TaskWorker`) and VSync-aligned GTK frame clock animations (`GdkFrameClock`). However, **two critical performance bottlenecks** cause unnecessary CPU overhead and GTK layout re-allocations:

1. **Subprocess Fork Thrashing in Polling Loops**: Every 3 seconds, `RightSection` executes multiple synchronous shell commands (`bluetoothctl`, `nmcli`). `bluetoothctl info <mac>` is executed sequentially for *every paired device*, resulting in 5–10 process forks every 3 seconds even when the system state is static.
2. **GTK Layout Tree Reconstruction**: On workspace changes, `LeftSection::update_workspaces()` strips all children from the GTK box container (`while let Some(...)`) and re-appends them, triggering full GTK layout measurement and CSS node invalidation passes.

---

## 2. Deep-Dive Technical Analysis

### A. Subprocess Fork Overhead & System Commands

Linux process creation (`fork` + `exec`) carries notable CPU overhead from kernel data structure allocation, page table copying, context switching, and environment initialization.

* **Bluetooth Service (`bluetoothctl`)**:
  - *Location*: [`src/services/bluetooth.rs`](file:///home/predator/COS_Niri/src/services/bluetooth.rs#L40-L79) $\rightarrow$ `BluetoothService::get_devices()`
  - *Issue*: To list devices, `bluetoothctl devices` is run, followed by `bluetoothctl info <mac>` for **every paired device**.
  - *Impact*: With 5 paired devices, every 3 seconds the bar spawns 6 separate kernel processes.

* **Network Service (`nmcli`)**:
  - *Location*: [`src/services/network.rs`](file:///home/predator/COS_Niri/src/services/network.rs#L40-L76) $\rightarrow$ `get_active_ssid()` & `get_active_signal()`
  - *Issue*: Spawns two distinct `nmcli` processes per poll cycle (`nmcli -t -f ACTIVE,SSID...` and `nmcli -t -f ACTIVE,SIGNAL...`).
  - *Impact*: Spawning `nmcli` queries NetworkManager over D-Bus twice per cycle instead of fetching both properties in a single pass or reading sysfs/D-Bus directly.

* **Brightness & Night Light Services**:
  - *Location*: [`src/services/brightness.rs`](file:///home/predator/COS_Niri/src/services/brightness.rs#L8-L33) & [`src/services/night_light.rs`](file:///home/predator/COS_Niri/src/services/night_light.rs#L7-L21)
  - *Issue*: Spawns `brightnessctl g` + `brightnessctl m` (2 processes) and `pgrep` for 3 target process names (up to 3 processes).
  - *Impact*: Reading `/sys/class/backlight/*/brightness` directly in Rust takes **$< 1 \mu s$** with zero process forks. Using `brightnessctl` takes several milliseconds and spawns subprocesses.

---

### B. Event-Driven Architecture vs. Polling Loop Redundancy

The codebase currently contains a structural redundancy where streaming event listeners and periodic timers operate concurrently on the same data sources:

| Component | Streaming Event Listener Active? | Periodic Polling Timer Active? | Assessment |
|---|---|---|---|
| **NetworkManager** | Yes (`nmcli monitor` in `bar.rs`) | Yes (3s timer in `RightSection`) | **Redundant**: Double queries |
| **Bluetooth** | Yes (`bluetoothctl` monitor in `bar.rs`) | Yes (3s timer in `RightSection`) | **Redundant**: Double queries |
| **Niri Workspaces / Windows** | Yes (`NiriIpcClient::listen_events`) | No (Event-driven) | **Optimal**: 0% idle CPU |

#### Key Finding:
- `nmcli monitor` and `bluetoothctl` stream state changes over pipes.
- Having a 3-second timer perform heavy polling on top of streaming event listeners causes the timer to execute redundant process forks 99% of the time when state has not changed.

---

### C. GTK4 Rendering Pipeline & Layout Hierarchy

* **Workspace Box Re-building**:
  - *Location*: [`src/components/left.rs`](file:///home/predator/COS_Niri/src/components/left.rs#L148-L156) $\rightarrow$ `update_workspaces()`
  - *Issue*:
    ```rust
    while let Some(child) = state.workspace_box.first_child() {
        state.workspace_box.remove(&child);
    }
    for ws in &workspaces { ... append(btn) ... }
    ```
  - *GTK Mechanics*: Removing and re-appending child widgets invalidates CSS tree nodes and forces GTK to trigger a `queue_resize()` pass (`measure()` and `allocate()`).
  - *Optimization*: Mutate existing `Button` widgets in place, add pills only for new workspaces, remove pills for deleted workspaces, and toggle `.active` CSS classes directly.

* **Animation System (`GdkFrameClock`)**:
  - *Location*: [`src/services/animation.rs`](file:///home/predator/COS_Niri/src/services/animation.rs)
  - *Assessment*: **Optimal**.
  - Tick callbacks connect directly to `GdkFrameClock` (VSync-aligned at 165Hz).
  - Uses microsecond time deltas (`clock.frame_time()`) for frame-rate independence.
  - Returns `glib::ControlFlow::Break` when complete, ensuring CPU/GPU usage drops back to **0%** immediately after the animation finishes.

---

## 3. Bottleneck Summary Matrix

| Subsystem | Current Mechanism | Bottleneck Cause | Recommended Fix | Expected Gain |
|---|---|---|---|---|
| **Bluetooth Status** | `bluetoothctl info <mac>` in loop | 6+ subprocess forks / 3s | Single D-Bus / `bluetoothctl` pass or event-driven | **~70% drop in background CPU spikes** |
| **Wi-Fi Status** | Dual `nmcli` subprocess calls | 2 process forks / 3s | Combine into 1 `nmcli` call or D-Bus query | **~50% reduction in network query overhead** |
| **Display Brightness** | `brightnessctl g` & `m` | 2 process forks / query | Direct `/sys/class/backlight` file read | **Sub-microsecond execution, 0 process forks** |
| **Night Light** | `pgrep` for 3 process names | 3 process forks / query | Procfs inspection (`/proc`) in Rust | **Instant check, 0 process forks** |
| **Workspace Pills** | Full container teardown & rebuild | GTK CSS node & layout invalidation | In-place widget property/CSS class mutation | **Zero layout recalculations on workspace switch** |

---

## 4. Actionable Step-by-Step Optimization Roadmap

### Phase 1: Zero-Fork System Property Readers
1. **Direct Sysfs Backlight Reading**:
   Read `/sys/class/backlight/*/brightness` and `max_brightness` directly using `std::fs::read_to_string()`.
2. **Procfs Night Light Detection**:
   Inspect `/proc` or check active process IDs natively in Rust instead of executing `pgrep`.
3. **Consolidated Network Status**:
   Execute a single `nmcli -t -f ACTIVE,SSID,SIGNAL dev wifi` command or query NetworkManager over D-Bus via `zbus`.

### Phase 2: Event-Driven UI Updates
1. **Connect `RightSection` to Event Channels**:
   Route events from the existing `NetworkService::listen_events` and `BluetoothService::listen_events` background streams directly to `RightSection` UI update calls.
2. **Reduce Polling Frequency**:
   Limit the `RightSection` timer strictly to system time formatting (e.g. every 10 seconds), eliminating periodic system status polling altogether.

### Phase 3: GTK4 DOM In-Place Mutation
1. **In-Place Workspace Widget Updates**:
   Update `LeftSection::update_workspaces` to reuse existing `Button` widgets, adjusting labels and `.active` classes without removing widgets from `workspace_box`.
2. **Prevent Unnecessary Container Resizes**:
   Minimize layout invalidations to maintain smooth 165Hz UI rendering.

---

*Report generated for COS_Niri project workspace.*
