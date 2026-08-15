use crate::components::center::CenterSection;
use crate::services::niri_config::NiriConfigService;
use crate::services::settings::SettingsService;
use crate::services::wallpaper::WallpaperService;
use gtk4::gdk;
use gtk4::gdk_pixbuf::Pixbuf;
use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box as GtkBox, Button, HeaderBar, Image, Label,
    Orientation, Picture, Scale, ScrolledWindow, SearchEntry, Separator, Stack,
    StackTransitionType, Switch,
};
use std::cell::RefCell;
use std::rc::Rc;

pub struct SettingsPopup {
    app: Application,
    window: Rc<RefCell<Option<ApplicationWindow>>>,
}

impl SettingsPopup {
    pub fn new(app: &Application) -> Self {
        Self {
            app: app.clone(),
            window: Rc::new(RefCell::new(None)),
        }
    }

    /// Build the window on demand
    fn build_window(
        app: &Application,
        win_holder: Rc<RefCell<Option<ApplicationWindow>>>,
    ) -> ApplicationWindow {
        // Create a standard GTK4 Application Window (Windowed App, NOT a Layer-Shell overlay)
        let window = ApplicationWindow::builder()
            .application(app)
            .title("Settings")
            .default_width(840)
            .default_height(560)
            .build();

        window.add_css_class("settings-window");

        // When closed by user (X titlebar button or window close), destroy and free all memory
        let holder_c = Rc::clone(&win_holder);
        let app_c = app.clone();
        window.connect_close_request(move |win| {
            let win_c = win.clone();
            let app_c2 = app_c.clone();
            let holder_c2 = Rc::clone(&holder_c);
            glib::idle_add_local_once(move || {
                win_c.set_child(None::<&gtk4::Widget>);
                win_c.set_titlebar(None::<&gtk4::Widget>);
                app_c2.remove_window(&win_c);
                win_c.destroy();
                *holder_c2.borrow_mut() = None;
                unsafe {
                    libc::malloc_trim(0);
                }
            });
            glib::Propagation::Proceed
        });

        // Sleek HeaderBar
        let header_bar = HeaderBar::new();
        header_bar.add_css_class("settings-headerbar");
        header_bar.set_show_title_buttons(true);
        window.set_titlebar(Some(&header_bar));

        // Main Shell Container (Sidebar + Content Area)
        let main_box = GtkBox::new(Orientation::Horizontal, 0);
        main_box.add_css_class("settings-main-container");
        main_box.set_hexpand(true);
        main_box.set_vexpand(true);

        let stack = Stack::new();
        stack.set_transition_type(StackTransitionType::Crossfade);
        stack.set_transition_duration(150);
        stack.set_hexpand(true);
        stack.set_vexpand(true);

        let stack_rc = Rc::new(RefCell::new(stack.clone()));

        // --- Pages (Core requested pages) ---
        let win_ref = window.clone();
        let appearance_page = Self::build_appearance_page(win_ref);
        let blur_page = Self::build_blur_page();
        let performance_page = Self::build_performance_page();
        let pinned_page = Self::build_pinned_apps_page();

        stack.add_named(&appearance_page, Some("appearance"));
        stack.add_named(&blur_page, Some("blur"));
        stack.add_named(&performance_page, Some("performance"));
        stack.add_named(&pinned_page, Some("pinned"));

        // --- Left Sidebar ---
        let nav_buttons = Rc::new(RefCell::new(Vec::new()));
        let sidebar = Self::build_sidebar(&stack_rc, &nav_buttons);
        main_box.append(&sidebar);

        // Sidebar Separator
        let sep = Separator::new(Orientation::Vertical);
        sep.add_css_class("settings-sidebar-sep");
        main_box.append(&sep);

        // Right Content Area
        let content_box = GtkBox::new(Orientation::Vertical, 0);
        content_box.add_css_class("settings-content-area");
        content_box.set_hexpand(true);
        content_box.set_vexpand(true);
        content_box.append(&stack);

        main_box.append(&content_box);
        window.set_child(Some(&main_box));

        window
    }

    /// Build the Sidebar navigation panel
    fn build_sidebar(
        stack_rc: &Rc<RefCell<Stack>>,
        nav_buttons: &Rc<RefCell<Vec<(String, Button)>>>,
    ) -> GtkBox {
        let sidebar = GtkBox::new(Orientation::Vertical, 4);
        sidebar.add_css_class("settings-sidebar");
        sidebar.set_size_request(210, -1);

        let nav_items = [
            ("appearance", "\u{e40a}", "Appearance"),
            ("blur", "\u{e3a5}", "Blur & Effects"),
            ("performance", "\u{e87b}", "Performance"),
            ("pinned", "\u{e148}", "Pinned Apps"),
        ];

        for (id, icon_code, label_text) in nav_items {
            let btn = Button::new();
            btn.add_css_class("settings-sidebar-btn");
            if id == "appearance" {
                btn.add_css_class("active");
            }

            let row = GtkBox::new(Orientation::Horizontal, 12);
            row.set_valign(gtk4::Align::Center);

            // Active bar indicator
            let indicator = GtkBox::new(Orientation::Vertical, 0);
            indicator.add_css_class("settings-nav-indicator");
            indicator.set_size_request(3, 20);
            row.append(&indicator);

            let icon = Label::new(Some(icon_code));
            icon.add_css_class("ms-icon");
            icon.add_css_class("settings-nav-icon");
            row.append(&icon);

            let lbl = Label::new(Some(label_text));
            lbl.add_css_class("settings-nav-label");
            lbl.set_hexpand(true);
            lbl.set_halign(gtk4::Align::Start);
            row.append(&lbl);

            btn.set_child(Some(&row));

            let st = Rc::clone(stack_rc);
            let btns_ref = Rc::clone(nav_buttons);
            let page_id = id.to_string();

            btn.connect_clicked(move |clicked_btn| {
                st.borrow().set_visible_child_name(&page_id);
                for (_, b) in btns_ref.borrow().iter() {
                    b.remove_css_class("active");
                }
                clicked_btn.add_css_class("active");
            });

            sidebar.append(&btn);
            nav_buttons.borrow_mut().push((id.to_string(), btn));
        }

        sidebar
    }

    /// Build the Appearance (Wallpaper & Matugen) Page
    fn build_appearance_page(win: ApplicationWindow) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 16);
        page.add_css_class("settings-page-body");

        // Page Header
        let hdr = GtkBox::new(Orientation::Horizontal, 10);
        let hdr_ic = Label::new(Some("\u{e40a}"));
        hdr_ic.add_css_class("ms-icon");
        hdr_ic.add_css_class("settings-page-header-icon");
        hdr.append(&hdr_ic);

        let hdr_lbl = Label::new(Some("Appearance & Wallpaper"));
        hdr_lbl.add_css_class("settings-page-header-title");
        hdr.append(&hdr_lbl);
        page.append(&hdr);

        // Card 1: Wallpaper Management with swaybg
        let wp_card = GtkBox::new(Orientation::Vertical, 12);
        wp_card.add_css_class("settings-card-container");

        let title = Label::new(Some("Desktop Wallpaper"));
        title.add_css_class("settings-card-title");
        title.set_halign(gtk4::Align::Start);
        wp_card.append(&title);

        let desc = Label::new(Some("Select a wallpaper to apply via swaybg and auto-generate Material You theme colors with Matugen."));
        desc.add_css_class("settings-card-desc");
        desc.set_wrap(true);
        desc.set_halign(gtk4::Align::Start);
        wp_card.append(&desc);

        // Wallpaper preview row
        let preview_row = GtkBox::new(Orientation::Horizontal, 16);
        preview_row.set_valign(gtk4::Align::Center);

        let picture = Picture::new();
        picture.add_css_class("settings-wallpaper-preview");
        picture.set_size_request(320, 180);
        picture.set_content_fit(gtk4::ContentFit::Cover);

        let wp_path = WallpaperService::get_current_path();
        let bg_path = dirs::home_dir().unwrap_or_default().join(".config/background");
        let active_path = if bg_path.exists() {
            Some(bg_path)
        } else if std::path::Path::new(&wp_path).exists() {
            Some(std::path::PathBuf::from(&wp_path))
        } else {
            None
        };

        if let Some(ref p) = active_path {
            if let Ok(pix) = Pixbuf::from_file_at_scale(p, 320, 180, true) {
                picture.set_paintable(Some(&gdk::Texture::for_pixbuf(&pix)));
            }
        }
        preview_row.append(&picture);

        let info_col = GtkBox::new(Orientation::Vertical, 10);
        info_col.set_hexpand(true);
        info_col.set_valign(gtk4::Align::Center);

        let path_lbl = Label::new(Some(&wp_path));
        path_lbl.add_css_class("settings-wallpaper-path");
        path_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
        path_lbl.set_max_width_chars(35);
        path_lbl.set_halign(gtk4::Align::Start);
        info_col.append(&path_lbl);

        let change_btn = Button::new();
        change_btn.add_css_class("settings-action-btn");
        change_btn.set_halign(gtk4::Align::Start);

        let btn_inner = GtkBox::new(Orientation::Horizontal, 8);
        btn_inner.set_halign(gtk4::Align::Center);
        let btn_ic = Label::new(Some("\u{e2c6}")); // add_photo_alternate
        btn_ic.add_css_class("ms-icon");
        btn_inner.append(&btn_ic);
        let btn_lbl = Label::new(Some("Change Wallpaper..."));
        btn_inner.append(&btn_lbl);
        change_btn.set_child(Some(&btn_inner));

        let pic_ref = picture.clone();
        let path_lbl_ref = path_lbl.clone();

        change_btn.connect_clicked(move |_| {
            let dialog = gtk4::FileDialog::new();
            dialog.set_title("Choose Wallpaper");

            let filter = gtk4::FileFilter::new();
            filter.set_name(Some("Images"));
            filter.add_mime_type("image/jpeg");
            filter.add_mime_type("image/png");
            filter.add_mime_type("image/webp");

            let filters = gio::ListStore::new::<gtk4::FileFilter>();
            filters.append(&filter);
            dialog.set_filters(Some(&filters));

            let pic_c = pic_ref.clone();
            let path_c = path_lbl_ref.clone();
            let w_c = win.clone();

            dialog.open(Some(&w_c), gio::Cancellable::NONE, move |result| {
                if let Ok(file) = result {
                    if let Some(path) = file.path() {
                        let path_str = path.to_string_lossy().to_string();
                        if let Ok(pix) = Pixbuf::from_file_at_scale(&path, 320, 180, true) {
                            pic_c.set_paintable(Some(&gdk::Texture::for_pixbuf(&pix)));
                        }
                        path_c.set_text(&path_str);

                        crate::services::worker::TaskWorker::dispatch(move || {
                            WallpaperService::set_wallpaper(&path_str);
                        });
                    }
                }
            });
        });

        info_col.append(&change_btn);
        preview_row.append(&info_col);
        wp_card.append(&preview_row);
        page.append(&wp_card);

        // Card 2: Material You 3 Theme Schemes
        let theme_card = GtkBox::new(Orientation::Vertical, 12);
        theme_card.add_css_class("settings-card-container");

        let t_title = Label::new(Some("Material You 3 Theme Scheme"));
        t_title.add_css_class("settings-card-title");
        t_title.set_halign(gtk4::Align::Start);
        theme_card.append(&t_title);

        let t_desc = Label::new(Some("Choose the color extraction algorithm for generating your system palette from the wallpaper."));
        t_desc.add_css_class("settings-card-desc");
        t_desc.set_wrap(true);
        t_desc.set_halign(gtk4::Align::Start);
        theme_card.append(&t_desc);

        let current_theme = SettingsService::get_theme();
        let schemes = [
            ("scheme-tonal-spot", "Tonal Spot", "\u{e40a}"),
            ("scheme-neutral", "Neutral", "\u{e3a5}"),
            ("scheme-vibrant", "Vibrant", "\u{e80e}"),
            ("scheme-expressive", "Expressive", "\u{e3b7}"),
            ("scheme-fidelity", "Fidelity", "\u{e8dc}"),
            ("scheme-rainbow", "Rainbow", "\u{e41d}"),
            ("scheme-fruit-salad", "Fruit Salad", "\u{e541}"),
            ("scheme-monochrome", "Monochrome", "\u{e3a8}"),
            ("scheme-content", "Content", "\u{e871}"),
        ];

        let scheme_grid = GtkBox::new(Orientation::Vertical, 6);
        let mut row_box = GtkBox::new(Orientation::Horizontal, 8);
        row_box.set_homogeneous(true);

        let scheme_buttons: Rc<RefCell<Vec<Button>>> = Rc::new(RefCell::new(Vec::new()));

        for (i, (s_id, s_name, s_icon)) in schemes.iter().enumerate() {
            if i > 0 && i % 3 == 0 {
                scheme_grid.append(&row_box);
                row_box = GtkBox::new(Orientation::Horizontal, 8);
                row_box.set_homogeneous(true);
            }

            let s_btn = Button::new();
            s_btn.add_css_class("settings-scheme-btn");
            if current_theme.scheme_type == *s_id {
                s_btn.add_css_class("active");
            }

            let btn_content = GtkBox::new(Orientation::Horizontal, 6);
            btn_content.set_halign(gtk4::Align::Center);
            btn_content.set_valign(gtk4::Align::Center);

            let ic = Label::new(Some(s_icon));
            ic.add_css_class("ms-icon");
            ic.add_css_class("ms-icon-sm");
            btn_content.append(&ic);

            let lbl = Label::new(Some(s_name));
            lbl.add_css_class("settings-scheme-label");
            btn_content.append(&lbl);

            s_btn.set_child(Some(&btn_content));

            let s_id_str = s_id.to_string();
            let btns_ref = Rc::clone(&scheme_buttons);

            s_btn.connect_clicked(move |clicked| {
                SettingsService::set_scheme_type(&s_id_str);
                for b in btns_ref.borrow().iter() {
                    b.remove_css_class("active");
                }
                clicked.add_css_class("active");
                crate::services::theme::ThemeService::regenerate();
            });

            row_box.append(&s_btn);
            scheme_buttons.borrow_mut().push(s_btn);
        }
        scheme_grid.append(&row_box);
        theme_card.append(&scheme_grid);

        // Dark / Light Mode Switch
        let mode_row = GtkBox::new(Orientation::Horizontal, 10);
        mode_row.add_css_class("settings-theme-mode-row");
        mode_row.set_valign(gtk4::Align::Center);

        let mode_info = GtkBox::new(Orientation::Vertical, 2);
        mode_info.set_hexpand(true);

        let mode_title = Label::new(Some("Dark Theme"));
        mode_title.add_css_class("settings-card-title");
        mode_title.set_halign(gtk4::Align::Start);
        mode_info.append(&mode_title);

        let mode_sub = Label::new(Some("Extract deep dark shades for surface and panels"));
        mode_sub.add_css_class("settings-card-sub");
        mode_sub.set_halign(gtk4::Align::Start);
        mode_info.append(&mode_sub);

        mode_row.append(&mode_info);

        let mode_sw = Switch::new();
        mode_sw.set_active(current_theme.dark_mode);
        mode_sw.set_valign(gtk4::Align::Center);

        mode_sw.connect_state_set(move |_, state| {
            SettingsService::set_dark_mode(state);
            crate::services::theme::ThemeService::regenerate();
            glib::Propagation::Proceed
        });
        mode_row.append(&mode_sw);

        theme_card.append(&mode_row);
        page.append(&theme_card);

        // Wrap in ScrolledWindow for comfortable viewing
        let scroll = ScrolledWindow::new();
        scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
        scroll.set_vexpand(true);
        scroll.set_child(Some(&page));

        let outer = GtkBox::new(Orientation::Vertical, 0);
        outer.set_hexpand(true);
        outer.set_vexpand(true);
        outer.append(&scroll);
        outer
    }

    /// Build the Blur & Effects Page
    fn build_blur_page() -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 16);
        page.add_css_class("settings-page-body");

        // Page Header
        let hdr = GtkBox::new(Orientation::Horizontal, 10);
        let hdr_ic = Label::new(Some("\u{e3a5}"));
        hdr_ic.add_css_class("ms-icon");
        hdr_ic.add_css_class("settings-page-header-icon");
        hdr.append(&hdr_ic);

        let hdr_lbl = Label::new(Some("Blur & Transparency"));
        hdr_lbl.add_css_class("settings-page-header-title");
        hdr.append(&hdr_lbl);
        page.append(&hdr);

        let card = GtkBox::new(Orientation::Vertical, 10);
        card.add_css_class("settings-card-container");

        let title = Label::new(Some("Module Blur & X-Ray Controls"));
        title.add_css_class("settings-card-title");
        title.set_halign(gtk4::Align::Start);
        card.append(&title);

        let desc = Label::new(Some("Toggle background blur or disable X-ray blur for particular shell modules in Niri rules. Changes apply immediately."));
        desc.add_css_class("settings-card-desc");
        desc.set_wrap(true);
        desc.set_halign(gtk4::Align::Start);
        card.append(&desc);

        let blur_settings = SettingsService::get_blur();
        let modules = [
            ("Bar Shelf", "bar", blur_settings.bar_blur, blur_settings.bar_xray),
            ("Quick Settings", "quick_settings", blur_settings.quick_settings_blur, blur_settings.quick_settings_xray),
            ("Calendar Popup", "calendar", blur_settings.calendar_blur, blur_settings.calendar_xray),
            ("App Launcher", "launcher", blur_settings.launcher_blur, blur_settings.launcher_xray),
            ("System Tray Menu", "tray", blur_settings.tray_blur, blur_settings.tray_xray),
        ];

        // Column header
        let hdr_row = GtkBox::new(Orientation::Horizontal, 0);
        hdr_row.add_css_class("settings-blur-header-row");

        let mod_hdr = Label::new(Some("Module Name"));
        mod_hdr.add_css_class("settings-blur-col-header");
        mod_hdr.set_hexpand(true);
        mod_hdr.set_halign(gtk4::Align::Start);
        hdr_row.append(&mod_hdr);

        let b_hdr = Label::new(Some("Blur"));
        b_hdr.add_css_class("settings-blur-col-header");
        b_hdr.set_width_chars(8);
        b_hdr.set_halign(gtk4::Align::Center);
        hdr_row.append(&b_hdr);

        let x_hdr = Label::new(Some("X-Ray"));
        x_hdr.add_css_class("settings-blur-col-header");
        x_hdr.set_width_chars(8);
        x_hdr.set_halign(gtk4::Align::Center);
        hdr_row.append(&x_hdr);
        card.append(&hdr_row);

        let blur_switches: Rc<RefCell<Vec<Switch>>> = Rc::new(RefCell::new(Vec::new()));
        let xray_switches: Rc<RefCell<Vec<Switch>>> = Rc::new(RefCell::new(Vec::new()));

        for (name, key, b_on, x_on) in modules {
            let row = GtkBox::new(Orientation::Horizontal, 8);
            row.add_css_class("settings-blur-row");
            row.set_valign(gtk4::Align::Center);

            let lbl = Label::new(Some(name));
            lbl.add_css_class("settings-blur-module-name");
            lbl.set_hexpand(true);
            lbl.set_halign(gtk4::Align::Start);
            row.append(&lbl);

            let b_sw = Switch::new();
            b_sw.set_active(b_on);
            let k_b = key.to_string();
            b_sw.connect_state_set(move |_, state| {
                NiriConfigService::set_blur(&k_b, state);
                glib::Propagation::Proceed
            });
            row.append(&b_sw);
            blur_switches.borrow_mut().push(b_sw);

            let x_sw = Switch::new();
            x_sw.set_active(x_on);
            let k_x = key.to_string();
            x_sw.connect_state_set(move |_, state| {
                NiriConfigService::set_xray(&k_x, state);
                glib::Propagation::Proceed
            });
            row.append(&x_sw);
            xray_switches.borrow_mut().push(x_sw);

            card.append(&row);
        }

        page.append(&card);

        // Card 2: Surface Opacity & Transparency Slider + Presets
        let op_card = GtkBox::new(Orientation::Vertical, 12);
        op_card.add_css_class("settings-card-container");

        let op_title = Label::new(Some("Surface Opacity & Transparency"));
        op_title.add_css_class("settings-card-title");
        op_title.set_halign(gtk4::Align::Start);
        op_card.append(&op_title);

        let op_desc = Label::new(Some("Control background surface opacity. Higher opacity (85-100%) produces solid, crisp surfaces when blur is turned off."));
        op_desc.add_css_class("settings-card-desc");
        op_desc.set_wrap(true);
        op_desc.set_halign(gtk4::Align::Start);
        op_card.append(&op_desc);

        let current_theme = SettingsService::get_theme();
        let current_opacity = current_theme.opacity;

        let slider_row = GtkBox::new(Orientation::Horizontal, 12);
        slider_row.set_valign(gtk4::Align::Center);

        let op_scale = Scale::with_range(Orientation::Horizontal, 10.0, 100.0, 1.0);
        op_scale.add_css_class("settings-volume-slider");
        op_scale.set_value(current_opacity as f64);
        op_scale.set_hexpand(true);
        slider_row.append(&op_scale);

        let op_val_label = Label::new(Some(&format!("{}%", current_opacity)));
        op_val_label.add_css_class("settings-volume-value");
        op_val_label.set_width_chars(4);
        slider_row.append(&op_val_label);
        op_card.append(&slider_row);

        // Opacity Presets Row
        let preset_row = GtkBox::new(Orientation::Horizontal, 8);
        preset_row.set_homogeneous(true);

        let op_presets = [
            (100, "Solid 100%"),
            (85, "Glass 85%"),
            (65, "Frosted 65%"),
            (45, "Translucent 45%"),
            (25, "Clear 25%"),
        ];

        let op_scale_c = op_scale.clone();
        let op_val_c = op_val_label.clone();
        let preset_buttons: Rc<RefCell<Vec<(u32, Button)>>> = Rc::new(RefCell::new(Vec::new()));

        let p_btns_ref = Rc::clone(&preset_buttons);
        op_scale.connect_value_changed(move |sc| {
            let val = sc.value().round() as u32;
            op_val_c.set_text(&format!("{}%", val));
            SettingsService::set_opacity(val);
            crate::services::theme::ThemeService::regenerate();

            for (p_val, b) in p_btns_ref.borrow().iter() {
                if *p_val == val {
                    b.add_css_class("active");
                } else {
                    b.remove_css_class("active");
                }
            }
        });

        for (p_val, p_name) in op_presets {
            let btn = Button::with_label(p_name);
            btn.add_css_class("settings-preset-btn");
            if current_opacity == p_val {
                btn.add_css_class("active");
            }

            let sc = op_scale_c.clone();
            btn.connect_clicked(move |_| {
                sc.set_value(p_val as f64);
            });

            preset_row.append(&btn);
            preset_buttons.borrow_mut().push((p_val, btn));
        }

        op_card.append(&preset_row);
        page.append(&op_card);

        // Card 3: Reset to Defaults Card
        let reset_card = GtkBox::new(Orientation::Vertical, 10);
        reset_card.add_css_class("settings-card-container");

        let r_title = Label::new(Some("Reset Configuration"));
        r_title.add_css_class("settings-card-title");
        r_title.set_halign(gtk4::Align::Start);
        reset_card.append(&r_title);

        let r_desc = Label::new(Some("Reset all blur rules, X-ray modes, and surface opacity back to default system settings."));
        r_desc.add_css_class("settings-card-desc");
        r_desc.set_wrap(true);
        r_desc.set_halign(gtk4::Align::Start);
        reset_card.append(&r_desc);

        let reset_btn = Button::new();
        reset_btn.add_css_class("settings-reset-btn");
        reset_btn.set_halign(gtk4::Align::Start);

        let r_btn_box = GtkBox::new(Orientation::Horizontal, 8);
        r_btn_box.set_halign(gtk4::Align::Center);
        let r_ic = Label::new(Some("\u{e5d5}")); // refresh
        r_ic.add_css_class("ms-icon");
        r_btn_box.append(&r_ic);
        let r_lbl = Label::new(Some("Reset to Defaults"));
        r_btn_box.append(&r_lbl);
        reset_btn.set_child(Some(&r_btn_box));

        let b_sws_ref = Rc::clone(&blur_switches);
        let x_sws_ref = Rc::clone(&xray_switches);
        let op_sc_ref = op_scale.clone();

        reset_btn.connect_clicked(move |_| {
            NiriConfigService::reset_rules();
            SettingsService::reset_to_defaults();
            crate::services::theme::ThemeService::regenerate();

            for sw in b_sws_ref.borrow().iter() {
                sw.set_active(true);
            }
            for sw in x_sws_ref.borrow().iter() {
                sw.set_active(false);
            }
            op_sc_ref.set_value(78.0);
        });

        reset_card.append(&reset_btn);
        page.append(&reset_card);

        // Wrap in ScrolledWindow for comfortable viewing
        let scroll = ScrolledWindow::new();
        scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
        scroll.set_vexpand(true);
        scroll.set_child(Some(&page));

        let outer = GtkBox::new(Orientation::Vertical, 0);
        outer.set_hexpand(true);
        outer.set_vexpand(true);
        outer.append(&scroll);
        outer
    }

    /// Build the Performance & Resource Optimization Page
    fn build_performance_page() -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 16);
        page.add_css_class("settings-page-body");

        // Page Header
        let header_box = GtkBox::new(Orientation::Vertical, 4);
        let title = Label::new(Some("Performance & Engine"));
        title.add_css_class("settings-section-title");
        title.set_halign(gtk4::Align::Start);
        header_box.append(&title);

        let subtitle = Label::new(Some("Configure rendering backend, 165Hz display tuning, and launcher resource mode."));
        subtitle.add_css_class("settings-section-sub");
        subtitle.set_halign(gtk4::Align::Start);
        header_box.append(&subtitle);
        page.append(&header_box);

        let perf = SettingsService::get_performance();

        // Card 1: Graphics Rendering Engine
        let engine_card = GtkBox::new(Orientation::Vertical, 10);
        engine_card.add_css_class("settings-card-container");

        let eng_title = Label::new(Some("Graphics Rendering Engine"));
        eng_title.add_css_class("settings-card-title");
        eng_title.set_halign(gtk4::Align::Start);
        engine_card.append(&eng_title);

        let eng_desc = Label::new(Some("Select the GTK Scene Kit (GSK) renderer. Vulkan delivers optimal frame pacing for 165Hz displays. (Requires bar restart to switch)."));
        eng_desc.add_css_class("settings-card-desc");
        eng_desc.set_halign(gtk4::Align::Start);
        eng_desc.set_wrap(true);
        engine_card.append(&eng_desc);

        let engines = [
            ("vulkan", "Vulkan (165Hz GPU)", "Full GPU acceleration with Wayland frame pacing. Recommended for high-refresh displays."),
            ("gl", "OpenGL (Balanced GPU)", "Fast hardware acceleration, lower driver compile overhead (~50 MB heap)."),
            ("cairo", "Cairo (Eco / Minimum RAM)", "Pure 2D software CPU rendering for ultra-low memory (~87 MB RSS)."),
        ];

        let engine_buttons: Rc<RefCell<Vec<(String, Button)>>> = Rc::new(RefCell::new(Vec::new()));

        for (id, label_text, desc_text) in engines {
            let btn = Button::new();
            btn.add_css_class("settings-engine-card");
            if perf.renderer == id {
                btn.add_css_class("active");
            }

            let b_box = GtkBox::new(Orientation::Vertical, 2);
            let b_title = Label::new(Some(label_text));
            b_title.add_css_class("settings-engine-title");
            b_title.set_halign(gtk4::Align::Start);

            let b_desc = Label::new(Some(desc_text));
            b_desc.add_css_class("settings-engine-desc");
            b_desc.set_halign(gtk4::Align::Start);
            b_desc.set_wrap(true);

            b_box.append(&b_title);
            b_box.append(&b_desc);
            btn.set_child(Some(&b_box));

            let id_str = id.to_string();
            let eb_ref = Rc::clone(&engine_buttons);
            btn.connect_clicked(move |_| {
                SettingsService::set_renderer(&id_str);
                for (eid, eb) in eb_ref.borrow().iter() {
                    if eid == &id_str {
                        eb.add_css_class("active");
                    } else {
                        eb.remove_css_class("active");
                    }
                }
            });

            engine_card.append(&btn);
            engine_buttons.borrow_mut().push((id.to_string(), btn));
        }

        page.append(&engine_card);

        // Card 2: App Launcher Backend
        let launcher_card = GtkBox::new(Orientation::Vertical, 10);
        launcher_card.add_css_class("settings-card-container");

        let l_title = Label::new(Some("App Launcher Mode"));
        l_title.add_css_class("settings-card-title");
        l_title.set_halign(gtk4::Align::Start);
        launcher_card.append(&l_title);

        let l_desc = Label::new(Some("Choose between the built-in glassmorphic drawer or instant standalone Fuzzel. (Applies immediately)."));
        l_desc.add_css_class("settings-card-desc");
        l_desc.set_halign(gtk4::Align::Start);
        l_desc.set_wrap(true);
        launcher_card.append(&l_desc);

        let launchers = [
            ("builtin", "Built-in App Drawer (Tahoe Grid)", "Glassmorphic macOS-style app grid. Pre-warmed for buttery 165Hz animations."),
            ("fuzzel", "Fuzzel (Instant & Zero RAM)", "Standalone launcher styled with Material You colors. Frees ~40 MB of shell memory."),
        ];

        let launcher_buttons: Rc<RefCell<Vec<(String, Button)>>> = Rc::new(RefCell::new(Vec::new()));

        for (id, label_text, desc_text) in launchers {
            let btn = Button::new();
            btn.add_css_class("settings-engine-card");
            if perf.launcher_backend == id {
                btn.add_css_class("active");
            }

            let b_box = GtkBox::new(Orientation::Vertical, 2);
            let b_title = Label::new(Some(label_text));
            b_title.add_css_class("settings-engine-title");
            b_title.set_halign(gtk4::Align::Start);

            let b_desc = Label::new(Some(desc_text));
            b_desc.add_css_class("settings-engine-desc");
            b_desc.set_halign(gtk4::Align::Start);
            b_desc.set_wrap(true);

            b_box.append(&b_title);
            b_box.append(&b_desc);
            btn.set_child(Some(&b_box));

            let id_str = id.to_string();
            let lb_ref = Rc::clone(&launcher_buttons);
            btn.connect_clicked(move |_| {
                SettingsService::set_launcher_backend(&id_str);
                for (lid, lb) in lb_ref.borrow().iter() {
                    if lid == &id_str {
                        lb.add_css_class("active");
                    } else {
                        lb.remove_css_class("active");
                    }
                }
            });

            launcher_card.append(&btn);
            launcher_buttons.borrow_mut().push((id.to_string(), btn));
        }

        page.append(&launcher_card);

        // Wrap in ScrolledWindow
        let scroll = ScrolledWindow::new();
        scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
        scroll.set_vexpand(true);
        scroll.set_child(Some(&page));

        let outer = GtkBox::new(Orientation::Vertical, 0);
        outer.set_hexpand(true);
        outer.set_vexpand(true);
        outer.append(&scroll);
        outer
    }

    /// Build the Pinned Apps Page
    fn build_pinned_apps_page() -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 12);
        page.add_css_class("settings-page-body");

        // Page Header
        let hdr = GtkBox::new(Orientation::Horizontal, 10);
        let hdr_ic = Label::new(Some("\u{e148}"));
        hdr_ic.add_css_class("ms-icon");
        hdr_ic.add_css_class("settings-page-header-icon");
        hdr.append(&hdr_ic);

        let hdr_lbl = Label::new(Some("Pinned Applications"));
        hdr_lbl.add_css_class("settings-page-header-title");
        hdr.append(&hdr_lbl);
        page.append(&hdr);

        let search = SearchEntry::new();
        search.add_css_class("settings-search");
        search.set_placeholder_text(Some("Search applications to pin..."));
        page.append(&search);

        let scroll = ScrolledWindow::new();
        scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
        scroll.set_vexpand(true);
        scroll.set_min_content_height(340);

        let list_box = GtkBox::new(Orientation::Vertical, 4);
        list_box.add_css_class("settings-pinned-list");
        scroll.set_child(Some(&list_box));
        page.append(&scroll);

        let list_rc = Rc::new(list_box);
        let search_rc = Rc::new(search.clone());

        let populate = {
            let list = Rc::clone(&list_rc);
            let s_in = Rc::clone(&search_rc);
            Rc::new(move || {
                Self::populate_pinned_list(&list, &s_in.text().to_string().to_lowercase());
            })
        };

        populate();

        let pop = Rc::clone(&populate);
        search.connect_search_changed(move |_| {
            pop();
        });

        page
    }

    fn populate_pinned_list(list_box: &GtkBox, query: &str) {
        while let Some(child) = list_box.first_child() {
            list_box.remove(&child);
        }

        let pinned = SettingsService::get_pinned_apps();
        let desktop_entries = CenterSection::scan_desktop_entries();
        let pinned_ids: Vec<String> = pinned.iter().map(|p| p.desktop_id.clone()).collect();

        // Currently Pinned
        let p_title = Label::new(Some("Currently Pinned"));
        p_title.add_css_class("settings-section-title");
        p_title.set_halign(gtk4::Align::Start);
        list_box.append(&p_title);

        for app in &pinned {
            let icon_name = desktop_entries
                .values()
                .find(|e| e.desktop_id == app.desktop_id)
                .map(|e| e.icon.clone())
                .unwrap_or_default();
            let row = Self::create_app_row(&app.name, &icon_name, &app.desktop_id, true, list_box);
            list_box.append(&row);
        }

        let sep = Separator::new(Orientation::Horizontal);
        sep.add_css_class("qs-sep");
        list_box.append(&sep);

        // Available Apps
        let a_title = Label::new(Some("Available Applications"));
        a_title.add_css_class("settings-section-title");
        a_title.set_halign(gtk4::Align::Start);
        list_box.append(&a_title);

        let mut available: Vec<_> = desktop_entries
            .values()
            .filter(|e| {
                if pinned_ids.contains(&e.desktop_id) {
                    return false;
                }
                if !query.is_empty() {
                    let k = format!("{} {}", e.name.to_lowercase(), e.desktop_id.to_lowercase());
                    if !k.contains(query) {
                        return false;
                    }
                }
                true
            })
            .collect();

        available.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        for entry in available.into_iter().take(30) {
            let row = Self::create_app_row(&entry.name, &entry.icon, &entry.desktop_id, false, list_box);
            list_box.append(&row);
        }
    }

    fn create_app_row(name: &str, icon_name: &str, desktop_id: &str, is_pinned: bool, list_box: &GtkBox) -> GtkBox {
        let row = GtkBox::new(Orientation::Horizontal, 10);
        row.add_css_class("settings-app-row");
        row.set_valign(gtk4::Align::Center);

        let icon = if icon_name.starts_with('/') && std::path::Path::new(icon_name).exists() {
            Image::from_file(icon_name)
        } else if !icon_name.is_empty() {
            Image::from_icon_name(icon_name)
        } else {
            Image::from_icon_name("application-x-executable")
        };
        icon.set_pixel_size(24);
        row.append(&icon);

        let lbl = Label::new(Some(name));
        lbl.add_css_class("settings-app-name");
        lbl.set_hexpand(true);
        lbl.set_halign(gtk4::Align::Start);
        row.append(&lbl);

        let btn = Button::new();
        btn.set_valign(gtk4::Align::Center);

        if is_pinned {
            btn.add_css_class("settings-unpin-btn");
            btn.set_child(Some(&Label::new(Some("Unpin"))));
            let did = desktop_id.to_string();
            let l_ref = list_box.clone();
            btn.connect_clicked(move |_| {
                SettingsService::unpin_app(&did);
                Self::populate_pinned_list(&l_ref, "");
                CenterSection::reload_pinned_apps();
            });
        } else {
            btn.add_css_class("settings-pin-btn");
            btn.set_child(Some(&Label::new(Some("Pin"))));
            let did = desktop_id.to_string();
            let n = name.to_string();
            let l_ref = list_box.clone();
            btn.connect_clicked(move |_| {
                SettingsService::pin_app(&did, &n);
                Self::populate_pinned_list(&l_ref, "");
                CenterSection::reload_pinned_apps();
            });
        }

        row.append(&btn);
        row
    }

    pub fn show(&self) {
        let mut win_slot = self.window.borrow_mut();
        if let Some(ref win) = *win_slot {
            win.present();
        } else {
            let win = Self::build_window(&self.app, Rc::clone(&self.window));
            win.present();
            *win_slot = Some(win);
        }
    }

    pub fn toggle(&self) {
        let mut win_slot = self.window.borrow_mut();
        if let Some(win) = win_slot.take() {
            // Already open -> destroy window, unparent and free memory
            win.set_child(None::<&gtk4::Widget>);
            self.app.remove_window(&win);
            win.destroy();
            unsafe {
                libc::malloc_trim(0);
            }
        } else {
            // Spawn on demand
            let win = Self::build_window(&self.app, Rc::clone(&self.window));
            win.present();
            *win_slot = Some(win);
        }
    }
}
