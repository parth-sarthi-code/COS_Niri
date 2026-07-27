use crate::services::audio::AudioService;
use crate::services::brightness::BrightnessService;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Label, Orientation, Scale};

pub struct SlidersSection {
    pub container: GtkBox,
}

impl SlidersSection {
    pub fn new<FAudio>(open_audio_page: FAudio) -> Self
    where
        FAudio: Fn() + Clone + 'static,
    {
        let container = GtkBox::new(Orientation::Vertical, 10);
        container.add_css_class("qs-sliders-section");

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
        vol_scale.connect_value_changed(|scale| {
            let val = scale.value().round() as u32;
            AudioService::set_volume(val);
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
        bright_scale.connect_value_changed(|scale| {
            let val = scale.value().round() as u32;
            BrightnessService::set_brightness(val);
        });
        bright_row.append(&bright_scale);

        // Spacer to balance the layout width with volume row arrow
        let dummy_spacer = GtkBox::new(Orientation::Horizontal, 0);
        dummy_spacer.set_size_request(32, 32);
        bright_row.append(&dummy_spacer);

        container.append(&bright_row);

        Self { container }
    }
}
