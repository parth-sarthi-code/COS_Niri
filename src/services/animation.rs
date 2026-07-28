use gtk4::prelude::*;
use std::sync::OnceLock;
use std::process::Command;

/// Cached display refresh rate in Hz (fetched once at startup)
static REFRESH_RATE: OnceLock<f64> = OnceLock::new();

/// Animation duration in milliseconds
const SLIDE_DURATION_MS: f64 = 280.0;

/// Slide distance in pixels (how far the popup travels upward)
const SLIDE_DISTANCE: f64 = 40.0;

/// Detect and cache the primary display refresh rate via `wlr-randr` or fallback
pub fn get_refresh_rate() -> f64 {
    *REFRESH_RATE.get_or_init(|| {
        // Try wlr-randr (Wayland / Niri native)
        if let Ok(output) = Command::new("wlr-randr").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let trimmed = line.trim();
                // Match lines like: "165.003006 Hz (preferred, current)"
                if trimmed.contains("Hz") && trimmed.contains("current") {
                    if let Some(hz_str) = trimmed.split_whitespace().next() {
                        if let Ok(hz) = hz_str.parse::<f64>() {
                            eprintln!("[animation] Detected refresh rate: {hz} Hz");
                            return hz;
                        }
                    }
                }
            }
        }

        // Try niri msg outputs
        if let Ok(output) = Command::new("niri").args(["msg", "outputs"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let trimmed = line.trim();
                if trimmed.contains("Hz") && trimmed.contains("current") {
                    // Parse "165.003 Hz (current)"
                    if let Some(hz_str) = trimmed.split_whitespace().next() {
                        if let Ok(hz) = hz_str.parse::<f64>() {
                            eprintln!("[animation] Detected refresh rate via niri: {hz} Hz");
                            return hz;
                        }
                    }
                }
            }
        }

        eprintln!("[animation] Using default refresh rate: 165 Hz");
        165.0
    })
}

/// Ease-out cubic: decelerating curve for natural-feeling slide-up
/// t is normalized progress [0.0, 1.0]
#[inline]
fn ease_out_cubic(t: f64) -> f64 {
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

    // Ensure refresh rate is cached
    let _ = get_refresh_rate();

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
        let progress = ease_out_cubic(raw_progress);

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

    // Duration for close is slightly faster for snappy feel
    let close_duration_ms = SLIDE_DURATION_MS * 0.75;

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
