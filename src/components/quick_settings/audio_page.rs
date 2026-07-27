use crate::services::audio::{AudioService, AudioSink};
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Label, Orientation, ScrolledWindow};
use std::sync::mpsc;
use std::thread;

pub struct AudioPage {
    pub container: GtkBox,
}

impl AudioPage {
    pub fn new<FBack>(on_back: FBack) -> Self
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

        let loading_lbl = Label::new(Some("Loading audio devices..."));
        loading_lbl.add_css_class("qs-subpage-empty");
        list_box.append(&loading_lbl);

        let (sender, receiver) = mpsc::channel::<Vec<AudioSink>>();
        let list_box_clone = list_box.clone();

        thread::spawn(move || {
            let sinks = AudioService::get_sinks();
            let _ = sender.send(sinks);
        });

        glib::idle_add_local(move || {
            if let Ok(sinks) = receiver.try_recv() {
                // Collect existing GTK Button items for recycling
                let mut existing_btns: Vec<Button> = Vec::new();
                let mut curr = list_box_clone.first_child();
                while let Some(child) = curr {
                    if let Ok(btn) = child.clone().downcast::<Button>() {
                        existing_btns.push(btn);
                    }
                    curr = child.next_sibling();
                }

                if !existing_btns.is_empty() && existing_btns.len() == sinks.len() {
                    // In-place widget recycling: update existing labels without destroying GTK widgets
                    for (i, sink) in sinks.into_iter().enumerate() {
                        let btn = &existing_btns[i];
                        if sink.is_default {
                            btn.add_css_class("active");
                        } else {
                            btn.remove_css_class("active");
                        }

                        if let Some(child_box) = btn.child().and_then(|c| c.downcast::<GtkBox>().ok()) {
                            let mut b_curr = child_box.first_child();
                            if let Some(icon) = b_curr {
                                b_curr = icon.next_sibling();
                            }
                            if let Some(desc_lbl) = b_curr.and_then(|c| c.downcast::<Label>().ok()) {
                                desc_lbl.set_text(&sink.description);
                            }
                        }

                        let sink_name = sink.name.clone();
                        btn.connect_clicked(move |_| {
                            AudioService::set_default_sink(&sink_name);
                        });
                    }
                } else {
                    // Rebuild list if element count changed
                    while let Some(child) = list_box_clone.first_child() {
                        list_box_clone.remove(&child);
                    }

                    for sink in sinks {
                        let item_btn = Button::new();
                        item_btn.add_css_class("qs-list-item");
                        if sink.is_default {
                            item_btn.add_css_class("active");
                        }

                        let row = GtkBox::new(Orientation::Horizontal, 8);
                        let icon = Label::new(Some("\u{e050}")); // speaker icon
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
                        item_btn.connect_clicked(move |_| {
                            AudioService::set_default_sink(&sink_name);
                        });

                        list_box_clone.append(&item_btn);
                    }
                }
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });

        scrolled.set_child(Some(&list_box));
        container.append(&scrolled);

        Self { container }
    }
}
