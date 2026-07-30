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