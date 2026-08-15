use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Label, Orientation};
use std::process::Command;

pub struct HeaderSection {
    pub container: GtkBox,
}

impl HeaderSection {
    pub fn new<F, FS>(on_collapse: F, on_settings: FS) -> Self
    where
        F: Fn() + 'static,
        FS: Fn() + 'static,
    {
        let container = GtkBox::new(Orientation::Horizontal, 8);
        container.add_css_class("qs-header");
        container.set_valign(gtk4::Align::Center);

        // Left user section: Avatar circle + Sign out pill
        let user_box = GtkBox::new(Orientation::Horizontal, 8);
        user_box.set_halign(gtk4::Align::Start);
        user_box.set_hexpand(true);
        user_box.set_valign(gtk4::Align::Center);

        let avatar_bubble = GtkBox::new(Orientation::Horizontal, 0);
        avatar_bubble.add_css_class("qs-avatar-bubble");
        avatar_bubble.set_size_request(32, 32);
        avatar_bubble.set_halign(gtk4::Align::Center);
        avatar_bubble.set_valign(gtk4::Align::Center);

        let avatar_icon = Label::new(Some("\u{e7fd}")); // person icon
        avatar_icon.add_css_class("ms-icon");
        avatar_icon.add_css_class("ms-icon-sm");
        avatar_icon.set_halign(gtk4::Align::Center);
        avatar_icon.set_valign(gtk4::Align::Center);
        avatar_icon.set_hexpand(true);
        avatar_icon.set_vexpand(true);
        avatar_bubble.append(&avatar_icon);
        user_box.append(&avatar_bubble);

        let signout_btn = Button::with_label("Sign out");
        signout_btn.add_css_class("qs-signout-btn");
        signout_btn.connect_clicked(|_| {
            let _ = Command::new("niri").args(["msg", "action", "quit"]).output();
        });
        user_box.append(&signout_btn);

        container.append(&user_box);

        // Right action icons: Power, Lock, Settings, Collapse
        let actions_box = GtkBox::new(Orientation::Horizontal, 4);
        actions_box.set_halign(gtk4::Align::End);
        actions_box.set_valign(gtk4::Align::Center);

        // Power button
        let power_btn = Self::create_action_icon("\u{e8ac}", "Power off");
        power_btn.connect_clicked(|_| {
            let _ = Command::new("systemctl").arg("poweroff").output();
        });
        actions_box.append(&power_btn);

        // Lock button
        let lock_btn = Self::create_action_icon("\u{e897}", "Lock screen");
        lock_btn.connect_clicked(|_| {
            let _ = Command::new("loginctl").arg("lock-session").output();
        });
        actions_box.append(&lock_btn);

        // Settings button — opens in-bar Settings popup
        let settings_btn = Self::create_action_icon("\u{e8b8}", "Settings");
        settings_btn.connect_clicked(move |_| {
            on_settings();
        });
        actions_box.append(&settings_btn);

        // Collapse chevron
        let collapse_btn = Self::create_action_icon("\u{e313}", "Collapse");
        collapse_btn.connect_clicked(move |_| {
            on_collapse();
        });
        actions_box.append(&collapse_btn);

        container.append(&actions_box);

        Self { container }
    }

    fn create_action_icon(icon_code: &str, tooltip: &str) -> Button {
        let btn = Button::new();
        btn.add_css_class("qs-header-icon-btn");
        btn.set_tooltip_text(Some(tooltip));

        let icon = Label::new(Some(icon_code));
        icon.add_css_class("ms-icon");
        icon.add_css_class("ms-icon-sm");
        btn.set_child(Some(&icon));
        btn
    }
}

