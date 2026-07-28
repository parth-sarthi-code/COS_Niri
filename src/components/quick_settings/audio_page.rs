use crate::services::audio::{AudioService, AudioSink};
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Label, Orientation, ScrolledWindow};
use std::rc::Rc;
use std::sync::mpsc;
use std::thread;

pub struct AudioPage {
    pub container: GtkBox,
    list_box: GtkBox,
}

impl AudioPage {
    pub fn new<FBack>(on_back: FBack) -> Rc<Self>
    where
        FBack: Fn() + 'static,
    {
        let container = GtkBox::new(Orientation::Vertical, 8);
        container.add_css_class("qs-subpage");

        // Header: Back Arrow + "Audio Output" Title
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

        let title = Label::new(Some("Audio Output"));
        title.add_css_class("qs-subpage-title");
        title.set_hexpand(true);
        title.set_halign(gtk4::Align::Start);
        header.append(&title);

        container.append(&header);

        // Audio Devices List inside ScrolledWindow
        let scrolled = ScrolledWindow::new();
        scrolled.set_min_content_height(220);
        scrolled.set_max_content_height(260);
        scrolled.add_css_class("qs-subpage-scroll");

        let list_box = GtkBox::new(Orientation::Vertical, 4);
        list_box.add_css_class("qs-subpage-list");

        scrolled.set_child(Some(&list_box));
        container.append(&scrolled);

        let page = Rc::new(Self {
            container,
            list_box,
        });

        page.sync_state();
        page
    }

    /// Asynchronously re-fetch PipeWire sinks and update device list live
    pub fn sync_state(&self) {
        let (sender, receiver) = mpsc::channel::<Vec<AudioSink>>();
        let list_box_clone = self.list_box.clone();

        thread::spawn(move || {
            let sinks = AudioService::get_sinks();
            let _ = sender.send(sinks);
        });

        glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            if let Ok(sinks) = receiver.try_recv() {
                // Clear existing GTK elements
                while let Some(child) = list_box_clone.first_child() {
                    list_box_clone.remove(&child);
                }

                if sinks.is_empty() {
                    let empty_lbl = Label::new(Some("No audio output devices found"));
                    empty_lbl.add_css_class("qs-subpage-empty");
                    list_box_clone.append(&empty_lbl);
                    return glib::ControlFlow::Break;
                }

                for sink in sinks {
                    let item_btn = Button::new();
                    item_btn.add_css_class("qs-list-item");
                    if sink.is_default {
                        item_btn.add_css_class("active");
                    }

                    let row = GtkBox::new(Orientation::Horizontal, 8);

                    // Smart contextual icon (Headphones vs DisplayPort/HDMI vs Speaker)
                    let icon_code = Self::get_sink_icon(&sink.name, &sink.description);
                    let icon = Label::new(Some(icon_code));
                    icon.add_css_class("ms-icon");
                    row.append(&icon);

                    let desc_lbl = Label::new(Some(&sink.description));
                    desc_lbl.add_css_class("qs-list-title");
                    desc_lbl.set_hexpand(true);
                    desc_lbl.set_halign(gtk4::Align::Start);
                    row.append(&desc_lbl);

                    if sink.is_default {
                        let check_icon = Label::new(Some("\u{e5ca}")); // check icon
                        check_icon.add_css_class("ms-icon");
                        row.append(&check_icon);
                    }

                    item_btn.set_child(Some(&row));

                    let sink_name = sink.name.clone();
                    let list_ref = list_box_clone.clone();

                    item_btn.connect_clicked(move |btn| {
                        // Optimistic UI state update
                        let mut curr = list_ref.first_child();
                        while let Some(child) = curr {
                            if let Ok(b) = child.clone().downcast::<Button>() {
                                b.remove_css_class("active");
                            }
                            curr = child.next_sibling();
                        }
                        btn.add_css_class("active");

                        AudioService::set_default_sink(&sink_name);
                    });

                    list_box_clone.append(&item_btn);
                }
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
    }

    /// Determine icon based on device type (Headphones, HDMI/DisplayPort, Speaker)
    fn get_sink_icon(name: &str, desc: &str) -> &'static str {
        let combined = format!("{name} {desc}").to_lowercase();
        if combined.contains("headphone") || combined.contains("headset") || combined.contains("earphone") {
            "\u{e30c}" // headphones icon
        } else if combined.contains("hdmi") || combined.contains("displayport") || combined.contains("tv") {
            "\u{e333}" // tv / display icon
        } else {
            "\u{e050}" // speaker icon
        }
    }
}
