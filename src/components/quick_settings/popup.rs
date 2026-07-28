use crate::components::quick_settings::audio_page::AudioPage;
use crate::components::quick_settings::bt_page::BtPage;
use crate::components::quick_settings::grid::GridSection;
use crate::components::quick_settings::header::HeaderSection;
use crate::components::quick_settings::sliders::SlidersSection;
use crate::components::quick_settings::wifi_page::WifiPage;
use crate::services::audio::AudioService;
use crate::services::battery::BatteryService;
use crate::services::brightness::BrightnessService;
use chrono::Local;
use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box as GtkBox, Label, Orientation, Separator, Stack,
    StackTransitionType,
};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

pub struct QuickSettingsPopup {
    pub window: ApplicationWindow,
    pub stack: Stack,
    pub grid: Rc<GridSection>,
    pub sliders: Rc<SlidersSection>,
    pub audio_page: Rc<AudioPage>,
    pub batt_label: Label,
    pub wifi_page: Rc<WifiPage>,
}

impl QuickSettingsPopup {
    pub fn new(app: &Application) -> Self {
        let window = ApplicationWindow::new(app);

        // Configure Layer-Shell popup window
        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_namespace("cos-quick-settings");

        // Anchor to Bottom-Right floating above the bar shelf
        window.set_anchor(Edge::Bottom, true);
        window.set_anchor(Edge::Right, true);
        window.set_margin(Edge::Bottom, 56);
        window.set_margin(Edge::Right, 12);

        window.add_css_class("qs-popup-window");

        let popup_box = GtkBox::new(Orientation::Vertical, 0);
        popup_box.add_css_class("qs-popup-container");
        popup_box.set_size_request(410, -1);

        let stack = Stack::new();
        stack.set_transition_type(StackTransitionType::SlideLeftRight);
        stack.set_transition_duration(200);

        let stack_rc = Rc::new(RefCell::new(stack.clone()));

        // --- SUB-PAGES ---
        let st_back1 = Rc::clone(&stack_rc);
        let wifi_page = Rc::new(WifiPage::new(move || {
            st_back1.borrow().set_visible_child_name("main");
        }));
        stack.add_named(&wifi_page.container, Some("wifi"));

        let st_back2 = Rc::clone(&stack_rc);
        let bt_page = BtPage::new(move || {
            st_back2.borrow().set_visible_child_name("main");
        });
        stack.add_named(&bt_page.container, Some("bt"));

        let st_back3 = Rc::clone(&stack_rc);
        let audio_page = AudioPage::new(move || {
            st_back3.borrow().set_visible_child_name("main");
        });
        stack.add_named(&audio_page.container, Some("audio"));

        // --- PAGE 0: Main ChromeOS View ---
        let main_view = GtkBox::new(Orientation::Vertical, 12);
        main_view.add_css_class("qs-main-view");

        // 1. Header (Avatar, Sign out, Power, Lock, Settings, Collapse)
        let s_close = window.clone();
        let header = HeaderSection::new(move || {
            s_close.set_visible(false);
        });
        main_view.append(&header.container);

        let sep1 = Separator::new(Orientation::Horizontal);
        sep1.add_css_class("qs-sep");
        main_view.append(&sep1);

        // 2. Feature Grid (6 Tiles)
        let st_wifi = Rc::clone(&stack_rc);
        let st_bt = Rc::clone(&stack_rc);
        let wp_open = Rc::clone(&wifi_page);

        let grid = Rc::new(GridSection::new(
            move || {
                wp_open.sync_state();
                st_wifi.borrow().set_visible_child_name("wifi");
            },
            move || {
                st_bt.borrow().set_visible_child_name("bt");
            },
        ));
        main_view.append(&grid.container);

        let sep2 = Separator::new(Orientation::Horizontal);
        sep2.add_css_class("qs-sep");
        main_view.append(&sep2);

        // 3. Sliders (Volume & Brightness)
        let st_audio = Rc::clone(&stack_rc);
        let ap_open = Rc::clone(&audio_page);
        let sliders = Rc::new(SlidersSection::new(move || {
            ap_open.sync_state();
            st_audio.borrow().set_visible_child_name("audio");
        }));
        main_view.append(&sliders.container);

        let sep3 = Separator::new(Orientation::Horizontal);
        sep3.add_css_class("qs-sep");
        main_view.append(&sep3);

        // 4. Footer (Date | Battery status)
        let footer = GtkBox::new(Orientation::Horizontal, 8);
        footer.add_css_class("qs-footer");
        footer.set_valign(gtk4::Align::Center);

        let date_str = Local::now().format("%a, %b %-d").to_string();
        let date_lbl = Label::new(Some(&date_str));
        date_lbl.add_css_class("qs-footer-text");
        footer.append(&date_lbl);

        let f_sep = Label::new(Some("|"));
        f_sep.add_css_class("qs-footer-sep");
        footer.append(&f_sep);

        let batt_info = BatteryService::get_info();
        let batt_str = if batt_info.is_present {
            format!("{}% - {}", batt_info.capacity, batt_info.status)
        } else {
            "Plugged in - AC Power".to_string()
        };

        let batt_lbl = Label::new(Some(&batt_str));
        batt_lbl.add_css_class("qs-footer-text");
        batt_lbl.set_hexpand(true);
        batt_lbl.set_halign(gtk4::Align::Start);
        footer.append(&batt_lbl);

        main_view.append(&footer);

        stack.add_named(&main_view, Some("main"));

        popup_box.append(&stack);
        window.set_child(Some(&popup_box));

        // --- EVENT LISTENERS (epoll sleep 0.0% CPU) ---
        // 1. Kernel inotify watcher for hardware/software brightness changes
        let (bright_tx, bright_rx) = mpsc::channel::<u32>();
        BrightnessService::listen_events(move |pct| {
            let _ = bright_tx.send(pct);
        });

        let s_bright = Rc::clone(&sliders);
        glib::timeout_add_local(Duration::from_millis(150), move || {
            let mut last_pct = None;
            while let Ok(pct) = bright_rx.try_recv() {
                last_pct = Some(pct);
            }
            if let Some(pct) = last_pct {
                s_bright.set_brightness_val(pct);
            }
            glib::ControlFlow::Continue
        });

        // 2. PipeWire audio stream listener for hardware volume / mute / sink hotplug events
        let (audio_tx, audio_rx) = mpsc::channel::<()>();
        AudioService::listen_events(move || {
            let _ = audio_tx.send(());
        });

        let s_audio = Rc::clone(&sliders);
        let ap_sync = Rc::clone(&audio_page);
        let stack_ref = stack.clone();
        glib::timeout_add_local(Duration::from_millis(150), move || {
            let mut fired = false;
            while audio_rx.try_recv().is_ok() {
                fired = true;
            }
            if fired {
                let (vol, is_muted) = AudioService::get_volume_and_mute();
                s_audio.set_volume_val(vol, is_muted);

                // Only re-fetch pactl list sinks if Audio sub-page is currently visible
                if stack_ref.visible_child_name().as_deref() == Some("audio") {
                    ap_sync.sync_state();
                }
            }
            glib::ControlFlow::Continue
        });

        Self {
            window,
            stack,
            grid,
            sliders,
            audio_page,
            batt_label: batt_lbl,
            wifi_page,
        }
    }

    /// Toggle visibility of the Quick Settings popup panel with slide animation
    pub fn toggle(&self) {
        use gtk4_layer_shell::LayerShell;
        if self.window.is_visible() {
            self.window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::None);
            crate::services::animation::slide_down_close(&self.window, 56);
        } else {
            self.window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::None);
            self.stack.set_visible_child_name("main");

            // Refresh sliders & mute status dynamically on panel open
            self.sliders.refresh();

            // 1. Trigger 165Hz slide-up animation IMMEDIATELY with 0 Wayland focus switches
            crate::services::animation::slide_up_open(&self.window, 56);

            // 2. Defer background queries until animation completes (280ms duration)
            let grid_ref = Rc::clone(&self.grid);
            let batt_ref = self.batt_label.clone();
            let wifi_page_ref = Rc::clone(&self.wifi_page);
            let audio_page_ref = Rc::clone(&self.audio_page);

            glib::timeout_add_local_once(std::time::Duration::from_millis(280), move || {
                let batt_info = BatteryService::get_info();
                let batt_str = if batt_info.is_present {
                    format!("{}% - {}", batt_info.capacity, batt_info.status)
                } else {
                    "Plugged in - AC Power".to_string()
                };
                batt_ref.set_text(&batt_str);
                GridSection::async_refresh(grid_ref);
                wifi_page_ref.sync_state();
                audio_page_ref.sync_state();
            });
        }
    }
}
