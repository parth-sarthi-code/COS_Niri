# Master TODO List

This document acts as a checklist to track development progress through Stage 1, Stage 2, and Stage 3 of the shell optimization roadmap.

## Stage 1: App Launcher Backend Fix & UI Revival/Optimization
- [x] **Desktop Exec Tokenizer**: Implement a robust tokenizer for `Exec` lines in [center.rs](file:///home/predator/COS_Niri/src/components/center.rs) that respects quotes, backslash escapes, and strips field codes (`%f`, `%u`, etc.).
- [x] **Lazy Population**: Delay `FlowBox` widget population in [launcher.rs](file:///home/predator/COS_Niri/src/components/launcher.rs) until the launcher window is opened for the first time.
- [x] **Native Grid Filtering**: Switch launcher search typing from widget recreation to GTK's native `FlowBox::set_filter_func` for zero-stutter search filtering.
- [x] **Tahoe Launcher DSA & UI Refactor**: Rebuild launcher in [launcher.rs](file:///home/predator/COS_Niri/src/components/launcher.rs) with a 5x4 pre-allocated widget recycling grid ($O(1)$ memory allocation), category pills bar, and floating glassmorphic container matching the Tahoe UI design.

## Stage 2: Leftover Backend Optimizations
- [x] **Brightness Latch Safety**: Wrapped the `listen_events` thread in [brightness.rs](file:///home/predator/COS_Niri/src/services/brightness.rs) with an RAII `ListeningGuard` to reset the global listening atomic flag on exit.
- [x] **Harden Worker Thread**: Implemented `catch_unwind` panic protection and automatic thread respawning in [worker.rs](file:///home/predator/COS_Niri/src/services/worker.rs).
- [x] **Off-thread Wallpaper Parsing**: Offloaded JPEG decoding and HSL quantization in [theme.rs](file:///home/predator/COS_Niri/src/services/theme.rs) to a background thread, firing `SIGUSR1` to hot-reload styles when ready.
- [x] **Async Font Caching**: Spawns `fc-cache -f` asynchronously in [main.rs](file:///home/predator/COS_Niri/src/main.rs) only if new font files were actually copied.
- [x] **Remove Hz Subprocess**: Stripped unused display refresh rate queries and subprocess spawns from [main.rs](file:///home/predator/COS_Niri/src/main.rs) and [animation.rs](file:///home/predator/COS_Niri/src/services/animation.rs).

## Stage 3: Panel Behavior & System Tray Integration
- [x] **Outside-Click Dismissal (Quick Settings)**: Native focus-loss listener auto-closes Quick Settings when clicking outside.
- [x] **Outside-Click Dismissal (Calendar)**: Native focus-loss listener auto-closes Calendar when clicking outside.
- [ ] **SNI Tray Service**: Build a D-Bus StatusNotifierItem watcher service in `src/services/tray.rs`.
- [ ] **Tray UI Integration**: Dynamically query, cache, and update tray icon widgets on the right side of the bar in [right.rs](file:///home/predator/COS_Niri/src/components/right.rs).
