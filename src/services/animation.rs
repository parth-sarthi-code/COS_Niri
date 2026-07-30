use gtk4::prelude::*;

/// GNOME Shell signature popover duration in milliseconds (220ms open for fluid motion)
const SLIDE_DURATION_MS: f64 = 220.0;

/// GNOME Shell popover travel distance in pixels (26px)
const SLIDE_DISTANCE: f64 = 26.0;

/// GNOME Shell custom cubic-bezier (0.25, 1.0, 0.5, 1.0) ease-out curve for buttery fluid motion
#[inline]
fn ease_out_gnome(t: f64) -> f64 {
    let t1 = 1.0 - t;
    1.0 - t1 * t1 * t1
}

/// Animate a Layer-Shell window sliding up from below + fading in.
///
/// Uses `widget.add_tick_callback()` which fires every VSync frame (165Hz = 165 fps).
/// Animation is time-based using `GdkFrameClock` timestamps, so it's
/// smooth and consistent regardless of actual frame rate.
///
/// - `window`: The popup window to animate
/// - `base_margin`: The resting bottom margin (e.g. 56px)
pub fn slide_up_open(window: &gtk4::ApplicationWindow, base_margin: i32) {
    use gtk4_layer_shell::LayerShell;

    // Set initial state: shifted down + transparent
    window.set_margin(gtk4_layer_shell::Edge::Bottom, base_margin - SLIDE_DISTANCE as i32);
    window.set_opacity(0.0);
    window.present();

    let start_time: std::cell::Cell<Option<i64>> = std::cell::Cell::new(None);
    let base = base_margin;
    let w = window.clone();

    window.add_tick_callback(move |_widget, clock| {
        let now_us = clock.frame_time(); // microseconds

        let start = match start_time.get() {
            Some(t) => t,
            None => {
                start_time.set(Some(now_us));
                now_us
            }
        };

        let elapsed_ms = (now_us - start) as f64 / 1000.0;
        let raw_progress = (elapsed_ms / SLIDE_DURATION_MS).min(1.0);
        let progress = ease_out_gnome(raw_progress);

        // Interpolate margin: start at (base - SLIDE_DISTANCE), end at base
        let current_margin = (base as f64 - SLIDE_DISTANCE * (1.0 - progress)) as i32;
        w.set_margin(gtk4_layer_shell::Edge::Bottom, current_margin);

        // Interpolate opacity: 0.0 → 1.0
        w.set_opacity(progress);

        if raw_progress >= 1.0 {
            // Animation complete — ensure final state is exact
            w.set_margin(gtk4_layer_shell::Edge::Bottom, base);
            w.set_opacity(1.0);
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });
}

/// Animate a Layer-Shell window sliding down + fading out, then hide.
///
/// Same tick-callback approach as slide_up_open but in reverse.
///
/// - `window`: The popup window to animate
/// - `base_margin`: The resting bottom margin (e.g. 56px)
pub fn slide_down_close(window: &gtk4::ApplicationWindow, base_margin: i32) {
    use gtk4_layer_shell::LayerShell;

    let w = window.clone();
    let start_time: std::cell::Cell<Option<i64>> = std::cell::Cell::new(None);
    let base = base_margin;

    // GNOME Shell signature close duration (170ms)
    let close_duration_ms = 170.0;

    window.add_tick_callback(move |_widget, clock| {
        let now_us = clock.frame_time();

        let start = match start_time.get() {
            Some(t) => t,
            None => {
                start_time.set(Some(now_us));
                now_us
            }
        };

        let elapsed_ms = (now_us - start) as f64 / 1000.0;
        let raw_progress = (elapsed_ms / close_duration_ms).min(1.0);
        // Ease-in for closing (accelerating away)
        let progress = raw_progress * raw_progress;

        // Slide down: base → (base - SLIDE_DISTANCE)
        let current_margin = (base as f64 - SLIDE_DISTANCE * progress) as i32;
        w.set_margin(gtk4_layer_shell::Edge::Bottom, current_margin);

        // Fade out: 1.0 → 0.0
        w.set_opacity(1.0 - progress);

        if raw_progress >= 1.0 {
            w.set_visible(false);
            // Reset to resting state for next open
            w.set_margin(gtk4_layer_shell::Edge::Bottom, base);
            w.set_opacity(1.0);
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });
}
