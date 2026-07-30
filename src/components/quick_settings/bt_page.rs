use crate::services::bluetooth::{BluetoothDevice, BluetoothService};
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Label, Orientation, ScrolledWindow, Switch};
use std::sync::mpsc;
use std::thread;

pub struct BtPage {
    pub container: GtkBox,
    pub toggle_switch: Switch,
    pub list_box: GtkBox,
}

impl BtPage {
    pub fn new<FBack>(on_back: FBack) -> Self
    where
        FBack: Fn() + 'static,
    {
        let container = GtkBox::new(Orientation::Vertical, 8);
        container.add_css_class("qs-subpage");

        // Header: Back Arrow + "Bluetooth" Title + Power Switch
        let header = GtkBox::new(Orientation::Horizontal, 8);
        header.add_css_class("qs-subpage-header");

        let back_btn = Button::new();
        back_btn.add_css_class("qs-header-icon-btn");
        let back_icon = Label::new(Some("\u{e5c4}")); // arrow_back
        back_icon.add_css_class("ms-icon");
        back_btn.set_child(Some(&back_icon));
        back_btn.connect_clicked(move |_| {
            on_back();
        });
        header.append(&back_btn);

        let title = Label::new(Some("Bluetooth"));
        title.add_css_class("qs-subpage-title");
        title.set_hexpand(true);
        title.set_halign(gtk4::Align::Start);
        header.append(&title);

        let is_on = BluetoothService::is_bluetooth_enabled();
        let toggle_switch = Switch::new();
        toggle_switch.set_active(is_on);

        header.append(&toggle_switch);
        container.append(&header);

        // Device List inside ScrolledWindow
        let scrolled = ScrolledWindow::new();
        scrolled.set_min_content_height(220);
        scrolled.set_max_content_height(260);
        scrolled.add_css_class("qs-subpage-scroll");

        let list_box = GtkBox::new(Orientation::Vertical, 4);
        list_box.add_css_class("qs-subpage-list");

        // Initial render
        Self::refresh_list(&list_box, is_on);

        // Switch toggle handler
        let list_box_clone = list_box.clone();
        toggle_switch.connect_state_set(move |_, state| {
            BluetoothService::set_bluetooth_enabled(state);
            Self::refresh_list(&list_box_clone, state);
            glib::Propagation::Proceed
        });

        scrolled.set_child(Some(&list_box));
        container.append(&scrolled);

        Self {
            container,
            toggle_switch,
            list_box,
        }
    }

    pub fn sync_state(&self) {
        let is_on = BluetoothService::is_bluetooth_enabled();
        self.toggle_switch.set_active(is_on);
        Self::refresh_list(&self.list_box, is_on);
    }

    fn refresh_list(list_box: &GtkBox, is_on: bool) {
        // Clear all list items
        while let Some(child) = list_box.first_child() {
            list_box.remove(&child);
        }

        if !is_on {
            let off_lbl = Label::new(Some("Bluetooth is turned off"));
            off_lbl.add_css_class("qs-subpage-empty");
            list_box.append(&off_lbl);
            return;
        }

        let loading_lbl = Label::new(Some("Searching Bluetooth devices..."));
        loading_lbl.add_css_class("qs-subpage-empty");
        list_box.append(&loading_lbl);

        let (sender, receiver) = mpsc::channel::<Vec<BluetoothDevice>>();
        let list_box_clone = list_box.clone();
        let (rfd, wfd) = crate::bar::create_event_pipe();

        thread::spawn(move || {
            let devices = BluetoothService::get_devices();
            let _ = sender.send(devices);
            crate::bar::notify_pipe(wfd);
            unsafe { libc::close(wfd); }
        });

        glib::unix_fd_add_local(rfd, glib::IOCondition::IN, move |fd, _| {
            crate::bar::drain_pipe(fd);
            if let Ok(devices) = receiver.try_recv() {
                while let Some(child) = list_box_clone.first_child() {
                    list_box_clone.remove(&child);
                }

                if devices.is_empty() {
                    let empty_lbl = Label::new(Some("No Bluetooth devices found"));
                    empty_lbl.add_css_class("qs-subpage-empty");
                    list_box_clone.append(&empty_lbl);
                } else {
                    for dev in devices {
                        let item_container = GtkBox::new(Orientation::Vertical, 0);
                        item_container.add_css_class("qs-list-item");

                        let row = GtkBox::new(Orientation::Horizontal, 8);
                        row.add_css_class("qs-list-item-row");

                        let icon = Label::new(Some("\u{e1a7}")); // bluetooth icon
                        icon.add_css_class("ms-icon");
                        row.append(&icon);

                        let name_lbl = Label::new(Some(&dev.name));
                        name_lbl.add_css_class("qs-list-title");
                        name_lbl.set_hexpand(true);
                        name_lbl.set_halign(gtk4::Align::Start);
                        row.append(&name_lbl);

                        if dev.is_connected {
                            let conn_lbl = Label::new(Some("Connected"));
                            conn_lbl.add_css_class("qs-list-connected");
                            row.append(&conn_lbl);
                        }

                        // Connect / Disconnect Action Button
                        let action_label = if dev.is_connected { "Disconnect" } else { "Connect" };
                        let action_btn = Button::with_label(action_label);
                        action_btn.add_css_class("qs-disc-btn");
                        action_btn.set_valign(gtk4::Align::Center);

                        let mac = dev.mac.clone();
                        let is_conn = dev.is_connected;
                        let lb_ref = list_box_clone.clone();
                        let btn_ref = action_btn.clone();
                        action_btn.connect_clicked(move |_| {
                            let loading_text = if is_conn { "Disconnecting..." } else { "Connecting..." };
                            btn_ref.set_label(loading_text);
                            btn_ref.set_sensitive(false);
                            BluetoothService::toggle_device_connection(&mac, is_conn);
                            
                            // Refresh list after a short delay
                            let lb_c = lb_ref.clone();
                            glib::timeout_add_seconds_local(2, move || {
                                Self::refresh_list(&lb_c, true);
                                glib::ControlFlow::Break
                            });
                        });
                        row.append(&action_btn);

                        item_container.append(&row);
                        list_box_clone.append(&item_container);
                    }
                }
            }
            unsafe { libc::close(fd); }
            glib::ControlFlow::Break
        });
    }
}
