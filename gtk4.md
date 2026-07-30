Switching to `gtk4-rs` changes the equation entirely—and massively in your favor. By writing your shell in Rust, you completely eliminate the GJS JavaScript garbage collector and interpreter overhead. This is the ultimate way to achieve native GNOME-level performance and zero-dropped-frame animations.

However, Rust’s strict concurrency and memory rules mean you must approach GTK4's single-threaded architecture carefully. Here is how to optimize for high-refresh-rate rendering and low CPU load using `gtk4-rs`.

## 1. Concurrency: The UI Thread Rule

GTK4 is strictly single-threaded. If you try to update a widget from a background system-polling thread (like checking CPU usage or battery), Rust won't just stutter—it will panic and crash your shell.

* **Never Block the Main Loop:** If you run heavy system commands (like `std::process::Command`) on the main thread, the UI will freeze and miss frame clock ticks, ruining your 144Hz animations.
* **Use GLib Channels:** To get data from background tasks (like async `tokio` loops monitoring DBus or Wayland) into the UI, use `glib::MainContext::channel()`. You spawn a background thread that sends messages, and you attach the receiver to the main context, which safely updates the widgets.
* **Master `glib::clone!`:** When passing widgets into closures (like button clicks or tick callbacks), standard Rust ownership rules clash with GTK's reference counting. Use the `glib::clone!` macro to create strong or weak references safely. Always use `#[upgrade_or]` to prevent panics if the widget has been destroyed.

## 2. Zero-Overhead State Bindings

Instead of using Astal's JavaScript accessors, `gtk4-rs` relies on native GLib properties.

* **Native Property Binding:** Use the `bind_property` builder. This offloads the state-syncing logic entirely to C, bypassing your Rust code entirely during execution.
```rust
// Example: Bind a slider's value directly to a label's text
slider.bind_property("value", &label, "label")
    .transform_to(|_, value: f64| Some(format!("{:.1}%", value).to_value()))
    .sync_create()
    .build();

```


* **Avoid Excessive Signals:** Do not use `connect_notify_local` if a simple `bind_property` will do. Emitting signals requires GLib to walk signal tables, which is fast, but native property bindings are faster.

## 3. High-Refresh Animations via Frame Clock

The concept of using the frame clock from the previous advice translates directly to Rust, but with strict return types.

* **`add_tick_callback`:** When animating custom drawn elements, attach a closure to the widget's frame clock. This provides a `gdk::FrameClock` that you can query for `frame_time()`.
* **Control Flow:** The closure must return `glib::ControlFlow::Continue` to keep the animation running for the next frame, or `glib::ControlFlow::Break` to stop it. Returning `Break` when the animation completes is crucial to drop CPU usage back to 0%.

## 4. Custom GSK Render Nodes in Rust

If you are building complex visualizers, do not use older Cairo drawing methods (`gtk::DrawingArea`) unless you absolutely need vector paths. Cairo uses software rendering (CPU).

* **Subclassing `WidgetImpl`:** To get maximum GPU performance, create a custom GObject subclass in Rust. Implement the `WidgetImpl` trait and override the `snapshot` method.
* **Building the Scene Graph:** Inside `snapshot`, you are given a `gtk::Snapshot`. Use it to append render nodes—like `append_color`, `append_texture`, or `append_linear_gradient`—directly to the GSK scene graph. The GPU renders these instantly.


To achieve the butter-smooth, high-refresh-rate animations seen in modern GNOME while keeping CPU and memory usage low, you have to understand how GTK4 and its rendering engine (GSK — GTK Scene Kit) operate under the hood.

In recent years, GNOME drastically improved its performance by shifting work away from the CPU (calculating layouts) and JavaScript execution, moving it directly to the GPU via scene graphs.

Here is how you can apply GNOME’s exact optimization strategies to your AGS/Astal widgets.

## 1. Animate Render Nodes, Not Layouts (The Golden Rule)

The most expensive thing you can do in GTK4 is trigger a "relayout" cycle. This happens when you change a widget's size, margin, or padding. The CPU has to walk through the entire widget tree and recalculate the geometry of every relative element.

If you animate a widget's position by changing its `margin-left` or `width` via CSS transitions, your CPU usage will spike, and the animation will stutter on high-refresh-rate monitors.

* **The GNOME Solution:** Use **CSS Transforms**. Animating `transform: translate(x, y);`, `scale()`, or `opacity` does not trigger a relayout. Instead, it simply updates a "Render Node" in GSK. The GPU handles this instantly with near-zero CPU cost.
* **In AGS:** Always use `transition: transform 200ms ease;` and apply translations instead of animating margins.

## 2. Keep JavaScript Out of the Animation Loop

JavaScript (GJS) execution is significantly slower than native C code. If you use a JS loop (`setInterval` or recursive timeouts) to animate a widget frame-by-frame, the GJS garbage collector will inevitably cause micro-stutters, ruining the fluidity on a 144Hz+ display.

* **Use Native Transition Widgets:** For expanding/collapsing UI elements, use `<revealer>` (`GtkRevealer`). For swapping out views, use `<stack>` (`GtkStack`). These widgets handle the animation entirely in C, syncing perfectly with the display's refresh rate without ever waking up the JavaScript engine.
* **CSS Transitions:** For color, hover states, and movement, define the `transition` purely in your CSS file and simply toggle a CSS class name from your JS.

## 3. Sync Custom Drawings to the `GdkFrameClock`

If you are building custom widgets (like an audio visualizer or a resource graph) that require continuous redrawing using Cairo, do not use generic timers to trigger redraws.

* **The Frame Clock:** GTK4 provides `Gdk.FrameClock`. This clock is strictly synced to the hardware vertical refresh rate of the monitor. It ensures that your code only paints when the monitor is actually ready to display a new frame, preventing screen tearing and wasted CPU cycles.
* **How to use it:** Attach a callback to the widget's frame clock (often via the `add_tick_callback` method in GTK). This callback gives you the exact timestamp of the frame, allowing you to calculate your animation math perfectly in sync with a 60Hz, 144Hz, or 240Hz display.

## 4. Leverage the New GSK Renderers (NGL/Vulkan)

GTK 4.14 introduced two highly optimized renderers: **NGL** (a new OpenGL renderer) and **Vulkan**. These utilize "ubershaders" to batch render nodes together, drastically reducing GPU memory bandwidth and improving fractional scaling performance.

While Astal/AGS handles the GTK integration, you can force your desktop to use these newer, faster renderers (if your system supports them) by launching your shell with environmental variables:

* `GSK_RENDERER=ngl` (The new default in GTK 4.14+; highly recommended for fluid 144Hz+ rendering).
* `GSK_RENDERER=vulkan` (Excellent for modern GPUs, though still receiving active patches).

## 5. Control Fractional Scaling Memory

If you use a high-resolution display with fractional scaling (e.g., 125% or 150%), rendering massive background images or unoptimized SVGs forces GTK to store massively upscaled textures in memory.

* **Downsample Assets:** Ensure your wallpapers and static icons are explicitly sized. Do not load a 4K image into a 300x300 widget without instructing the widget to downscale it on load, otherwise GTK will hold the full 4K texture in GPU memory.


Building high-performance desktop widgets with AGS (Aylur's GTK Shell) and its Astal/Gnim framework requires shifting away from typical web-development habits. Because Gnim JSX is not React—it is purely syntactic sugar for declarative `GObject.Object` construction—optimizing your GTK4 widgets means focusing on efficient state bindings, proper memory management, and respecting GTK4's native rendering rules.

Here is a guide to optimizing your GTK4 widgets built with AGS/Astal.

## 1. Efficient State Management & Bindings

Avoid heavy polling or standard web-framework hooks. Instead, use AGS's accessor primitives to manage state directly tied to GObject properties.

* **Use `createBinding` for Native Properties:** Instead of setting up listeners that update a generic state variable, bind directly to the GObject property. `createBinding(gobject, "property")` creates an accessor hooked directly into the object, bypassing unnecessary JavaScript event-loop overhead.
* **Derive State with `createComputed`:** When a value depends on other states (like combining a volume integer and a mute boolean to pick an icon), use `createComputed(() => value)`. This ensures the UI only recalculates the result when the underlying accessors change.
* **Read State Natively:** Always read state by calling the accessor as a function (e.g., `count()`). This creates the reactive dependency without triggering unnecessary whole-component re-renders.

## 2. Layout and Control Flow Rules

GTK4 handles widget creation and destruction differently than a Virtual DOM. Improperly rendering dynamic elements is the most common cause of layout thrashing and high CPU usage.

* **Use `<list>` for Dynamic Arrays:** When rendering things like workspaces or notifications, use the native `<list>` tag. It optimizes how items are added or removed from the DOM.
* **Use `<switch>` for Conditionals:** For conditional rendering or unwrapping nullable objects, use the `<switch>` tag.
* **The Container Rule:** You must **always** wrap `<list>` and `<switch>` tags inside a static container widget (like an intrinsic `<box>`). When items change inside these tags, previous widgets are removed and new ones are appended. If they aren't enclosed in a dedicated container, the visual order of your shell will break as widgets pop into the wrong positions.

## 3. GTK4 Specific Rendering Quirks

GTK4 changed several core windowing and visibility rules from GTK3. Addressing these prevents "invisible widget" bugs and keeps rendering fast.

* **Explicit Visibility:** Unlike GTK3, widgets in GTK4 are invisible by default. You must explicitly apply the `visible` attribute to your intrinsic widgets for them to render.
* **Window Size Allocation:** A `GtkWindow` will not render if it doesn't have an initial size allocation upon construction. For example, if your window's direct child is a `<revealer>` starting with `reveal_child: false`, the window collapses to zero size and won't appear. **Fix:** Wrap children in a `<box>` and assign that box a minimum CSS size so the window compositor knows how to map it.
* **Application Singletons:** Window instances must be strictly bound to the application singleton to properly manage their lifecycle. Always assign it via the property `application={app}` (or via a setup function like `setup={(self) => app.add_window(self)}`).

## 4. Signals and Event Handling

* **Native Signal Handlers:** Attach signal handlers directly using the `on` prefix (e.g., `onClicked`, `onNotifyChildRevealed`). This directly connects to the GTK signal infrastructure, executing callbacks immediately without passing through a synthetic event system.
* **Avoid Deep Nesting:** GTK4 offloads rendering to the GPU using scene graphs (GSK). While highly optimized, deeply nested `<box>` widgets force the layout manager to perform complex geometry calculations on every resize. Keep your widget trees as shallow as possible.