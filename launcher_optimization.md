# GTK4 & App Launcher CPU Optimization Report

This report analyzes the CPU utilization patterns of `cos-niri-bar` during active UI navigation and details the rendering pipeline of GTK4, along with concrete scopes for optimization.

---

## 1. Analysis of CPU Utilization during UI Navigation

In the resource consumption profiles, we observe:
*   **Idle / Steady State**: ~0.0% to 3.8% CPU.
*   **Active UI Navigation (Scrolling / Hovering / Searching)**: Spikes up to **15.0% - 24.1%** CPU.

### Is this normal for GTK4?
**Yes, this is normal and expected behavior.** Here is why:

1.  **Layout Reflow is CPU-Bound**: GTK4 uses GPU rendering (via Vulkan or OpenGL/NGL) to *rasterize* and *paint* pixels. However, the **layout engine** (measuring widget dimensions, computing borders, calculating text wrapping, and building the Scene Graph render tree) still runs entirely on the **CPU main thread**.
2.  **State Invalidation**: Hovering over grid items or scrolling invalidates widget states (changing styles like margins, background opacities, or border-radius). This forces GTK's CSS engine to recalculate style nodes and rebuild the render nodes.
3.  **High Refresh Rates (120Hz/144Hz)**: On modern displays, Wayland delivers pointer events and frame ticks at high refresh rates. Moving the mouse rapidly can trigger up to 120–144 state updates and rendering cycles per second on the CPU.

---

## 2. Identified Scopes for Code Optimization

Although the baseline CPU spikes are normal for GTK4, we can optimize the launcher implementation to reduce redundant layout reflows and CPU cycles:

### Scope A: Search Input Debouncing (High Impact)
*   **Current Issue**: Typing a search query triggers the full `refresh_grid()` instantly on every single keystroke. If a user types "chrome" quickly, the grid is filtered, sorted, and rebuilt 6 times in rapid succession.
*   **Optimization**: Implement input debouncing using `glib::timeout_add_local_once`. Wait **100ms** after the last keystroke before invoking `refresh_grid()`. This prevents redundant filtering and widget updates during active typing.

### Scope B: Cache Label Text updates (Medium Impact)
*   **Current Issue**: In `refresh_grid()`, `tile.label.set_text(&entry.name)` is called unconditionally for every visible tile, even if the application name is identical. Calling `set_text` invalidates the Pango layout cache and forces a CPU layout reflow for the text node.
*   **Optimization**: Cache the last set text in a `last_name: Rc<RefCell<String>>` field on `TileWidget`. Only call `set_text` if the name has actually changed.

### Scope C: Lazy/Asynchronous Icon Loading (Medium Impact)
*   **Current Issue**: When displaying applications, GTK looks up and loads icons from the system icon theme (`tile.icon.set_icon_name`) or reads them from path files on the main thread. If many icons change at once, this causes brief main thread blockages.
*   **Optimization**: Load icons asynchronously via a worker thread, or pre-load and cache the scaled `GIcon` objects during startup inside the desktop entry cache.

### Scope D: Event Bubbling Optimization (Low Impact)
*   **Current Issue**: Scrolling in a grid of widgets triggers a high density of motion-notify and hover state updates.
*   **Optimization**: Set `can-focus` to `false` or adjust target hover transition times in CSS to be slightly wider to reduce CSS style recalculation frequency.
