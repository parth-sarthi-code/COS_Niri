use gtk4::cairo::{RectangleInt, Region};
use gtk4::gdk::Display;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Box as GtkBox, GestureClick, Orientation};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::rc::Rc;

/// Bar shelf height in pixels — subtracted from the click shield's input region
/// so pointer events on bar widgets pass through to the bar surface.
const BAR_SHELF_HEIGHT: i32 = 48;

pub struct ClickCatcher {
    pub window: ApplicationWindow,
}

impl ClickCatcher {
    pub fn new<F>(app: &Application, on_dismiss: F) -> Self
    where
        F: Fn() + 'static,
    {
        let window = ApplicationWindow::new(app);

        // ── Layer-Shell Configuration ──
        // Matching Noctalia v5: shield lives on the SAME layer as the popups (Top).
        // Within a layer, wlr-layer-shell stacks surfaces in mapping order:
        // the shield is mapped BEFORE the popup, so the popup renders on top.
        // Clicks landing on the popup area hit the popup surface (above the shield).
        // Clicks landing elsewhere hit the shield's input region → dismiss.
        window.init_layer_shell();
        window.set_layer(Layer::Top);
        window.set_namespace("cos-click-catcher");

        // Anchor to all four edges → fullscreen coverage
        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Bottom, true);
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Right, true);

        // exclusive_zone(-1): overlap everything including the bar's exclusive zone.
        // The bar shelf area is excluded from the input region (see apply_input_region),
        // so clicks on bar widgets still flow directly to the bar surface.
        window.set_exclusive_zone(-1);

        // KeyboardMode::None is the correct mode for Niri.
        // Niri delivers pointer events to layer-shell surfaces regardless of
        // keyboard interactivity, so None preserves focus-follows-mouse behavior
        // without stealing keyboard focus from app windows.
        window.set_keyboard_mode(KeyboardMode::None);

        window.add_css_class("click-catcher-window");

        // Transparent child widget (GTK requires a child for the surface to have content)
        let overlay = GtkBox::new(Orientation::Vertical, 0);
        overlay.set_hexpand(true);
        overlay.set_vexpand(true);
        overlay.add_css_class("click-catcher-overlay");
        window.set_child(Some(&overlay));

        // ── Dismiss Gesture ──
        let dismiss_callback = Rc::new(on_dismiss);
        let gesture = GestureClick::new();
        gesture.set_button(0);

        let cb_clone = Rc::clone(&dismiss_callback);
        gesture.connect_released(move |_, _, _, _| {
            cb_clone();
        });

        window.add_controller(gesture);

        // ── Input Region: Applied After Realize ──
        let w_realize = window.clone();
        window.connect_realize(move |_| {
            Self::apply_input_region(&w_realize);
        });

        Self { window }
    }

    /// Query the primary monitor geometry in application pixels.
    /// Returns (width, height) or None if no monitor is found.
    fn monitor_geometry() -> Option<(i32, i32)> {
        let display = Display::default()?;
        let monitors = display.monitors();
        let monitor = monitors
            .item(0)
            .and_then(|m| m.downcast::<gtk4::gdk::Monitor>().ok())?;
        let geom = monitor.geometry();
        Some((geom.width(), geom.height()))
    }

    /// Apply the Wayland input region to the underlying GDK surface.
    ///
    /// Matching Noctalia v5's `applyInputRegion()`:
    /// 1. Create a region covering the full monitor dimensions
    /// 2. Subtract the bottom bar shelf rectangle (48px)
    /// 3. Apply to the GDK surface via `set_input_region()`
    ///
    /// We use monitor geometry (not surface.width/height) because the GDK surface
    /// may report stale 1×1 dimensions before the compositor sends configure.
    fn apply_input_region(window: &ApplicationWindow) {
        let Some(surface) = window.surface() else {
            return;
        };

        // Use monitor geometry as the authoritative surface size.
        // The click catcher is anchored to all four edges with exclusive_zone(-1),
        // so its logical size equals the monitor's logical size.
        let Some((width, height)) = Self::monitor_geometry() else {
            return;
        };

        if width <= 0 || height <= 0 {
            return;
        }

        // Full-screen input region
        let full_rect = RectangleInt::new(0, 0, width, height);
        let region = Region::create_rectangle(&full_rect);

        // Subtract the bar shelf at the bottom edge
        let bar_rect = RectangleInt::new(0, height - BAR_SHELF_HEIGHT, width, BAR_SHELF_HEIGHT);
        let _ = region.subtract_rectangle(&bar_rect);

        surface.set_input_region(&region);
    }

    /// Show the click shield. Must be called BEFORE the popup's `present()`
    /// to guarantee correct within-layer stacking order (shield below popup).
    pub fn show(&self) {
        self.window.set_visible(true);
        self.window.present();

        // Apply input region after present(). We use monitor geometry
        // so dimensions are always correct even before configure arrives.
        Self::apply_input_region(&self.window);
    }

    pub fn hide(&self) {
        self.window.set_visible(false);
    }
}
