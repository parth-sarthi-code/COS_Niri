# COS_Niri - Comprehensive Backend Optimization Scope & Architecture Roadmap

This document outlines a complete technical audit of all backend services, IPC socket handlers, concurrency models, and memory allocations in `cos-niri-bar`, detailing actionable optimization strategies for maximum responsiveness, minimum CPU/RAM footprint, and zero latency.

---

## 1. Architectural Upgrade: Native D-Bus Integration vs CLI Subprocess Spawning

### Current State
Backend services (`audio.rs`, `bluetooth.rs`, `brightness.rs`, `network.rs`) currently execute external CLI utilities (`pactl`, `wpctl`, `nmcli`, `bluetoothctl`, `brightnessctl`) via `std::process::Command::new(...)`.

### Optimization Scope
- **Subprocess Overhead**: Each CLI call requires kernel process forking (`fork()`, `execve()`), environment initialization, stdin/stdout pipe creation, and string parsing of human-readable output (~10ms–30ms per invocation).
- **Direct D-Bus IPC Solution**:
  - Replace `nmcli` with direct D-Bus calls to `org.freedesktop.NetworkManager`.
  - Replace `bluetoothctl` with direct D-Bus calls to BlueZ (`org.bluez`).
  - Replace `pactl`/`wpctl` with direct PipeWire/PulseAudio D-Bus / native C library bindings.
- **Performance Gains**:
  - **Latency**: Reduces command execution latency from ~20ms down to **<0.1ms** (pure in-memory Unix domain socket message passing).
  - **Memory & CPU**: Eliminates 100% of process fork allocations and string buffer allocations from stdout.

---

## 2. Concurrency & Threading Model: Reusable Worker Queue vs Transient OS Threads

### Current State
When sliders are moved, switches are toggled, or sub-pages query device lists, new OS threads are spawned via `std::thread::spawn(move || { ... })`.

### Optimization Scope
- **OS Thread Allocation**: Spawning a native OS thread creates a new kernel thread stack (typically allocating 2 MB of virtual memory per thread) and causes kernel context-switching overhead. On rapid slider drags (e.g. volume from 0 to 100), dozens of transient threads are created and destroyed in milliseconds.
- **Worker Queue Solution**:
  - Implement a single, persistent background **Worker Thread Channel** (`std::sync::mpsc::channel`) or an async executor (`tokio` single-threaded runtime).
  - Incoming requests (e.g. `SetVolume(80)`, `ToggleBluetooth(true)`) are pushed to a channel queue and consumed by 1 persistent background worker thread.
- **Performance Gains**:
  - **Zero Thread Creation Overhead**: Reuses a single background thread indefinitely.
  - **Command Coalescing / Debouncing**: If 10 volume slider updates arrive in 5ms, the worker queue coalesces them into a single final volume update command, saving 90% of redundant execution.

---

## 3. Desktop Entry Scanner & Flatpak App Caching (`src/components/center.rs`)

### Current State
`scan_desktop_entries()` scans `/usr/share/applications`, `/var/lib/flatpak/exports/...`, and `~/.local/share/applications`, opening and parsing `.desktop` text files to resolve application icons and launch commands.

### Optimization Scope
- **File System I/O**: Scanning and parsing dozens of `.desktop` entry files on filesystem directories can incur unnecessary disk reads if invoked repeatedly.
- **In-Memory Cache Solution**:
  - Wrap desktop entry index in an in-memory thread-safe cache (`once_cell::sync::Lazy<RwLock<HashMap<String, DesktopEntry>>>` or `Arc<HashMap>`).
  - Scan desktop files **once at app startup** and watch XDG directory changes via `inotify` or `gio::FileMonitor` only when new applications are installed/removed.
- **Performance Gains**:
  - **0ms App Launcher Resolution**: Fuzzel-style app icon and command resolution becomes a pure in-memory `HashMap` lookup (<0.001ms).

---

## 4. GTK Widget Recycling & Scene Graph Optimization (`subpages`)

### Current State
In `wifi_page.rs`, `bt_page.rs`, and `audio_page.rs`, opening a sub-page clears all children (`list_box.remove(&child)`) and constructs new GTK `Button`, `Box`, and `Label` widgets for every network or device.

### Optimization Scope
- **Widget Allocation & GC**: Destroying and re-creating GTK4 widget objects triggers GTK CSS parsing, layout nodes allocation, and main thread garbage cleanup.
- **Incremental Widget Recycling Solution**:
  - Store existing list items in a `HashMap<String, (Button, Label, Label)>` keyed by SSID, MAC address, or Sink Name (identical to the pattern used in `left.rs` workspace pills).
  - When new scan data arrives, update text/CSS classes on existing widgets, adding new items or removing missing ones incrementally.
- **Performance Gains**:
  - **Zero Frame Drops during Scans**: Sub-page updates execute with 0 widget allocation pressure on GTK 4's render graph.

---

## 5. String Allocation & Parsing Efficiency

### Current State
String parsing operations (`to_string()`, `split_whitespace()`, `String::from_utf8_lossy`) allocate heap memory during device scanning and line parsing.

### Optimization Scope
- **Zero-Copy Parsing**:
  - Utilize borrowed string slices (`&str`) during line splitting and tokenization.
  - Pre-allocate vector capacities (`Vec::with_capacity(capacity)`) in `get_sinks()`, `get_devices()`, and `scan_networks()` to prevent vector reallocation during scanning.

---

## Summary Matrix of Proposed Optimizations

| Area | Current Approach | Optimized Approach | Expected Performance Gain |
| :--- | :--- | :--- | :--- |
| **System Services** | CLI Process Execution (`nmcli`, `pactl`) | Direct D-Bus IPC (`org.bluez`, NetworkManager) | Latency: ~20ms → **<0.1ms** |
| **Threading** | `std::thread::spawn` per action | Dedicated Worker Queue / Coalesced Channel | **0 Thread Creation Overhead** |
| **Desktop Entries** | File parsing on demand | In-Memory `Lazy<HashMap>` + `inotify` | App Lookup: **0ms** |
| **GTK Sub-Pages** | Destroy & Re-create Widgets | Widget Recycling (`HashMap<ID, Widget>`) | **0 Rendering Frame Drops** |
| **String Handling** | Allocating `String` operations | Borrowed Slices (`&str`) + Pre-allocated Vecs | **Minimal Memory Churn** |
