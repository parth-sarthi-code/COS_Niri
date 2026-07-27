use crate::services::bluetooth::{BluetoothDevice, BluetoothService};
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Label, Orientation, ScrolledWindow, Switch};
use std::sync::mpsc;
use std::thread;

pub struct BtPage {
    pub container: GtkBox,
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
        toggle_switch.connect_state_set(|_, state| {
            BluetoothService::set_bluetooth_enabled(state);
            glib::Propagation::Proceed
        });
        header.append(&toggle_switch);

        container.append(&header);

        // Device List inside ScrolledWindow
        let scrolled = ScrolledWindow::new();
        scrolled.set_min_content_height(220);
        scrolled.set_max_content_height(260);
        scrolled.add_css_class("qs-subpage-scroll");

        let list_box = GtkBox::new(Orientation::Vertical, 4);
        list_box.add_css_class("qs-subpage-list");

        if is_on {
            let loading_lbl = Label::new(Some("Searching Bluetooth devices..."));
            loading_lbl.add_css_class("qs-subpage-empty");
            list_box.append(&loading_lbl);

            let (sender, receiver) = mpsc::channel::<Vec<BluetoothDevice>>();
            let list_box_clone = list_box.clone();

            thread::spawn(move || {
                let devices = BluetoothService::get_devices();
                let _ = sender.send(devices);
            });

            glib::idle_add_local(move || {
                if let Ok(devices) = receiver.try_recv() {
                    // Collect existing GTK Button items for recycling
                    let mut existing_btns: Vec<Button> = Vec::new();
                    let mut curr = list_box_clone.first_child();
                    while let Some(child) = curr {
                        if let Ok(btn) = child.clone().downcast::<Button>() {
                            existing_btns.push(btn);
                        }
                        curr = child.next_sibling();
                    }

                    if !existing_btns.is_empty() && existing_btns.len() == devices.len() {
                        // In-place widget recycling: update existing labels without destroying GTK widgets
                        for (i, dev) in devices.into_iter().enumerate() {
                            let btn = &existing_btns[i];
                            if let Some(child_box) = btn.child().and_then(|c| c.downcast::<GtkBox>().ok()) {
                                let mut b_curr = child_box.first_child();
                                if let Some(icon) = b_curr {
                                    b_curr = icon.next_sibling();
                                }
                                if let Some(name_lbl) = b_curr.and_then(|c| c.downcast::<Label>().ok()) {
                                    name_lbl.set_text(&dev.name);
                                }
                            }
                            let mac = dev.mac.clone();
                            let is_conn = dev.is_connected;
                            btn.connect_clicked(move |_| {
                                BluetoothService::toggle_device_connection(&mac, is_conn);
                            });
                        }
                    } else {
                        // Rebuild list if element count changed
                        while let Some(child) = list_box_clone.first_child() {
                            list_box_clone.remove(&child);
                        }

                        if devices.is_empty() {
                            let empty_lbl = Label::new(Some("No Bluetooth devices found"));
                            empty_lbl.add_css_class("qs-subpage-empty");
                            list_box_clone.append(&empty_lbl);
                        } else {
                            for dev in devices {
                                let item_btn = Button::new();
                                item_btn.add_css_class("qs-list-item");

                                let row = GtkBox::new(Orientation::Horizontal, 8);
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

                                item_btn.set_child(Some(&row));

                                let mac = dev.mac.clone();
                                let is_conn = dev.is_connected;
                                item_btn.connect_clicked(move |_| {
                                    BluetoothService::toggle_device_connection(&mac, is_conn);
                                });

                                list_box_clone.append(&item_btn);
                            }
                        }
                    }
                    glib::ControlFlow::Break
                } else {
                    glib::ControlFlow::Continue
                }
            });
        } else {
            let off_lbl = Label::new(Some("Bluetooth is turned off"));
            off_lbl.add_css_class("qs-subpage-empty");
            list_box.append(&off_lbl);
        }

        scrolled.set_child(Some(&list_box));
        container.append(&scrolled);

        Self { container }
    }
}
