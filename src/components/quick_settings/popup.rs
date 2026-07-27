use crate::components::quick_settings::audio_page::AudioPage;
use crate::components::quick_settings::bt_page::BtPage;
use crate::components::quick_settings::grid::GridSection;
use crate::components::quick_settings::header::HeaderSection;
use crate::components::quick_settings::sliders::SlidersSection;
use crate::components::quick_settings::wifi_page::WifiPage;
use chrono::Local;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Box as GtkBox, Label, Orientation, Separator, Stack, StackTransitionType};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::cell::RefCell;
use std::rc::Rc;

pub struct QuickSettingsPopup {
    pub window: ApplicationWindow,
    pub stack: Stack,
    pub grid: Rc<GridSection>,
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

        let grid = Rc::new(GridSection::new(
            move || {
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
        let sliders = SlidersSection::new(move || {
            st_audio.borrow().set_visible_child_name("audio");
        });
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

        let batt_lbl = Label::new(Some("100% - Battery"));
        batt_lbl.add_css_class("qs-footer-text");
        batt_lbl.set_hexpand(true);
        batt_lbl.set_halign(gtk4::Align::Start);
        footer.append(&batt_lbl);

        main_view.append(&footer);

        stack.add_named(&main_view, Some("main"));

        // --- SUB-PAGES ---
        let st_back1 = Rc::clone(&stack_rc);
        let wifi_page = WifiPage::new(move || {
            st_back1.borrow().set_visible_child_name("main");
        });
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

        popup_box.append(&stack);
        window.set_child(Some(&popup_box));

        Self { window, stack, grid }
    }

    /// Toggle visibility of the Quick Settings popup panel instantly (0ms presentation)
    pub fn toggle(&self) {
        if self.window.is_visible() {
            self.window.set_visible(false);
        } else {
            self.stack.set_visible_child_name("main");
            self.window.present();
            GridSection::async_refresh(Rc::clone(&self.grid));
        }
    }
}
