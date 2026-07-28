use crate::components::calendar::popup::CalendarPopup;
use crate::components::center::CenterSection;
use crate::components::left::LeftSection;
use crate::components::quick_settings::grid::GridSection;
use crate::components::quick_settings::popup::QuickSettingsPopup;
use crate::components::right::RightSection;
use crate::services::bluetooth::BluetoothService;
use crate::services::network::NetworkService;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Box as GtkBox, Orientation, Separator};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::rc::Rc;
use std::sync::mpsc;

#[allow(dead_code)]
pub struct BarWindow {
    pub window: ApplicationWindow,
    pub left_section: LeftSection,
    pub center_section: CenterSection,
    pub right_section: RightSection,
    pub quick_settings: Rc<QuickSettingsPopup>,
    pub calendar: Rc<CalendarPopup>,
}

impl BarWindow {
    pub fn new(app: &Application) -> Self {
        let window = ApplicationWindow::new(app);

        // Configure Layer-Shell for Wayland (Niri)
        window.init_layer_shell();
        window.set_layer(Layer::Top);
        window.set_namespace("cos-bar");

        // Anchor to bottom edge spanning full width
        window.set_anchor(Edge::Bottom, true);
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Right, true);

        // Reserve exclusive zone so Niri tiles windows above the bar
        window.set_exclusive_zone(48);

        window.add_css_class("cos-bar-window");

        // Instantiate Quick Settings floating popup
        let quick_settings = Rc::new(QuickSettingsPopup::new(app));

        // Instantiate Calendar floating popup
        let calendar = Rc::new(CalendarPopup::new(app));

        // NetworkManager live event listener via channel (event-driven)
        let (net_tx, net_rx) = mpsc::channel::<()>();
        NetworkService::listen_events(move || {
            let _ = net_tx.send(());
        });

        let qs_net = Rc::clone(&quick_settings);
        glib::timeout_add_seconds_local(1, move || {
            while net_rx.try_recv().is_ok() {
                GridSection::async_refresh(Rc::clone(&qs_net.grid));
            }
            glib::ControlFlow::Continue
        });

        // Bluetooth live event listener via channel (event-driven)
        let (bt_tx, bt_rx) = mpsc::channel::<()>();
        BluetoothService::listen_events(move || {
            let _ = bt_tx.send(());
        });

        let qs_bt = Rc::clone(&quick_settings);
        glib::timeout_add_seconds_local(1, move || {
            while bt_rx.try_recv().is_ok() {
                GridSection::async_refresh(Rc::clone(&qs_bt.grid));
            }
            glib::ControlFlow::Continue
        });

        // Main shelf container
        let main_box = GtkBox::new(Orientation::Horizontal, 0);
        main_box.add_css_class("cos-bar-container");

        let left_section = LeftSection::new();
        let center_section = CenterSection::new();

        let qs_toggle_ref = Rc::clone(&quick_settings);
        let cal_toggle_ref = Rc::clone(&calendar);

        let right_section = RightSection::new(
            move || {
                qs_toggle_ref.toggle();
            },
            move || {
                cal_toggle_ref.toggle();
            },
        );

        // Left — fixed to start
        left_section.container.set_halign(gtk4::Align::Start);
        left_section.container.set_hexpand(false);
        main_box.append(&left_section.container);

        // Vertical separator between left and center
        let sep_lc = Separator::new(Orientation::Vertical);
        sep_lc.add_css_class("shelf-sep");
        main_box.append(&sep_lc);

        // Center — expand and center in available space
        let center_wrapper = GtkBox::new(Orientation::Horizontal, 0);
        center_wrapper.set_hexpand(true);
        center_wrapper.set_halign(gtk4::Align::Center);
        center_wrapper.append(&center_section.container);
        main_box.append(&center_wrapper);

        // Vertical separator between center and right
        let sep_cr = Separator::new(Orientation::Vertical);
        sep_cr.add_css_class("shelf-sep");
        main_box.append(&sep_cr);

        // Right — fixed to end
        right_section.container.set_halign(gtk4::Align::End);
        right_section.container.set_hexpand(false);
        main_box.append(&right_section.container);

        window.set_child(Some(&main_box));

        Self {
            window,
            left_section,
            center_section,
            right_section,
            quick_settings,
            calendar,
        }
    }

    pub fn show(&self) {
        self.window.present();
    }
}
