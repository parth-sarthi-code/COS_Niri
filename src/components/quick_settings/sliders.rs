use crate::services::audio::AudioService;
use crate::services::brightness::BrightnessService;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Label, Orientation, Scale};
use std::cell::Cell;
use std::rc::Rc;

pub struct SlidersSection {
    pub container: GtkBox,
    vol_scale: Scale,
    vol_label: Label,
    bright_scale: Scale,
    is_updating: Rc<Cell<bool>>,
}

impl SlidersSection {
    pub fn new<FAudio>(open_audio_page: FAudio) -> Self
    where
        FAudio: Fn() + Clone + 'static,
    {
        let container = GtkBox::new(Orientation::Vertical, 10);
        container.add_css_class("qs-sliders-section");

        let is_updating = Rc::new(Cell::new(false));

        // 1. Volume Row
        let vol_row = GtkBox::new(Orientation::Horizontal, 8);
        vol_row.set_valign(gtk4::Align::Center);

        let vol_icon_code = if AudioService::is_muted() { "\u{e04f}" } else { "\u{e050}" };
        let vol_btn = Button::new();
        vol_btn.add_css_class("qs-slider-icon-btn");
        let vol_label = Label::new(Some(vol_icon_code));
        vol_label.add_css_class("ms-icon");
        vol_btn.set_child(Some(&vol_label));
        vol_btn.connect_clicked(|_| {
            AudioService::toggle_mute();
        });
        vol_row.append(&vol_btn);

        let curr_vol = AudioService::get_volume() as f64;
        let vol_scale = Scale::with_range(Orientation::Horizontal, 0.0, 100.0, 1.0);
        vol_scale.set_value(curr_vol);
        vol_scale.add_css_class("qs-slider");
        vol_scale.set_hexpand(true);
        vol_scale.set_valign(gtk4::Align::Center);

        let is_up_vol = Rc::clone(&is_updating);
        vol_scale.connect_value_changed(move |scale| {
            if !is_up_vol.get() {
                let val = scale.value().round() as u32;
                AudioService::set_volume(val);
            }
        });
        vol_row.append(&vol_scale);

        let audio_page_btn = Button::new();
        audio_page_btn.add_css_class("qs-slider-arrow-btn");
        let arrow_icon = Label::new(Some("\u{e5cc}")); // chevron_right
        arrow_icon.add_css_class("ms-icon");
        audio_page_btn.set_child(Some(&arrow_icon));
        audio_page_btn.connect_clicked(move |_| {
            open_audio_page();
        });
        vol_row.append(&audio_page_btn);

        container.append(&vol_row);

        // 2. Brightness Row
        let bright_row = GtkBox::new(Orientation::Horizontal, 8);
        bright_row.set_valign(gtk4::Align::Center);

        let bright_btn = Button::new();
        bright_btn.add_css_class("qs-slider-icon-btn");
        let bright_label = Label::new(Some("\u{e518}")); // light_mode
        bright_label.add_css_class("ms-icon");
        bright_btn.set_child(Some(&bright_label));
        bright_row.append(&bright_btn);

        let curr_bright = BrightnessService::get_brightness() as f64;
        let bright_scale = Scale::with_range(Orientation::Horizontal, 0.0, 100.0, 1.0);
        bright_scale.set_value(curr_bright);
        bright_scale.add_css_class("qs-slider");
        bright_scale.set_hexpand(true);
        bright_scale.set_valign(gtk4::Align::Center);

        let is_up_bright = Rc::clone(&is_updating);
        bright_scale.connect_value_changed(move |scale| {
            if !is_up_bright.get() {
                let val = scale.value().round() as u32;
                BrightnessService::set_brightness(val);
            }
        });
        bright_row.append(&bright_scale);

        let dummy_spacer = GtkBox::new(Orientation::Horizontal, 0);
        dummy_spacer.set_size_request(32, 32);
        bright_row.append(&dummy_spacer);

        container.append(&bright_row);

        Self {
            container,
            vol_scale,
            vol_label,
            bright_scale,
            is_updating,
        }
    }

    /// Dynamically refresh sliders and mute icon
    pub fn refresh(&self) {
        self.is_updating.set(true);

        let vol = AudioService::get_volume() as f64;
        self.vol_scale.set_value(vol);

        let icon = if AudioService::is_muted() { "\u{e04f}" } else { "\u{e050}" };
        if self.vol_label.text() != icon {
            self.vol_label.set_text(icon);
        }

        let bright = BrightnessService::get_brightness() as f64;
        self.bright_scale.set_value(bright);

        self.is_updating.set(false);
    }

    /// Update brightness scale directly from inotify signal
    pub fn set_brightness_val(&self, pct: u32) {
        self.is_updating.set(true);
        self.bright_scale.set_value(pct as f64);
        self.is_updating.set(false);
    }

    /// Update volume scale directly from audio signal
    pub fn set_volume_val(&self, pct: u32, is_muted: bool) {
        self.is_updating.set(true);
        self.vol_scale.set_value(pct as f64);
        let icon = if is_muted { "\u{e04f}" } else { "\u{e050}" };
        if self.vol_label.text() != icon {
            self.vol_label.set_text(icon);
        }
        self.is_updating.set(false);
    }
}
