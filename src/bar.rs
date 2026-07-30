use crate::components::calendar::popup::CalendarPopup;
use crate::components::center::CenterSection;
use crate::components::click_catcher::ClickCatcher;
use crate::components::left::LeftSection;
use crate::components::launcher::LauncherPopup;
use crate::components::quick_settings::grid::GridSection;
use crate::components::quick_settings::popup::QuickSettingsPopup;
use crate::components::right::RightSection;
use crate::services::bluetooth::BluetoothService;
use crate::services::network::NetworkService;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Box as GtkBox, Orientation, Separator};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::cell::RefCell;
use std::os::unix::io::RawFd;
use std::rc::Rc;

/// Create a Unix pipe pair for zero-poll event notification.
/// Returns (read_fd, write_fd). Both are set to O_CLOEXEC.
pub fn create_event_pipe() -> (RawFd, RawFd) {
    let mut fds = [0i32; 2];
    unsafe {
        libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC | libc::O_NONBLOCK);
    }
    (fds[0], fds[1])
}

/// Write a single byte to the pipe to wake the GLib main loop.
pub fn notify_pipe(write_fd: RawFd) {
    unsafe {
        libc::write(write_fd, [1u8].as_ptr() as *const _, 1);
    }
}

/// Drain all pending bytes from the pipe read end (coalesces multiple events).
pub fn drain_pipe(read_fd: RawFd) {
    let mut buf = [0u8; 64];
    unsafe {
        while libc::read(read_fd, buf.as_mut_ptr() as *mut _, buf.len()) > 0 {}
    }
}

#[allow(dead_code)]
pub struct BarWindow {
    pub window: ApplicationWindow,
    pub left_section: LeftSection,
    pub center_section: CenterSection,
    pub right_section: Rc<RightSection>,
    pub quick_settings: Rc<QuickSettingsPopup>,
    pub calendar: Rc<CalendarPopup>,
    pub launcher: Rc<LauncherPopup>,
    pub click_catcher: Rc<ClickCatcher>,
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

        // Instantiate App Launcher popup
        let launcher = Rc::new(LauncherPopup::new(app));

        // Instantiate Quick Settings floating popup
        let quick_settings = Rc::new(QuickSettingsPopup::new(app));

        // Instantiate Calendar floating popup
        let calendar = Rc::new(CalendarPopup::new(app));

        // Transparent full-screen Layer-Shell overlay for outside-click dismissal (matching Noctalia v5)
        let l_dismiss = Rc::clone(&launcher);
        let q_dismiss = Rc::clone(&quick_settings);
        let c_dismiss = Rc::clone(&calendar);
        let r_dismiss_cell = Rc::new(RefCell::new(None::<Rc<RightSection>>));
        let r_dismiss_ref = Rc::clone(&r_dismiss_cell);
        let cc_cell = Rc::new(RefCell::new(Option::<Rc<ClickCatcher>>::None));
        let cc_dismiss_ref = Rc::clone(&cc_cell);

        let click_catcher = Rc::new(ClickCatcher::new(app, move || {
            l_dismiss.close();
            q_dismiss.close();
            c_dismiss.close();
            if let Some(ref r) = *r_dismiss_ref.borrow() {
                r.close();
            }
            if let Some(ref cc) = *cc_dismiss_ref.borrow() {
                cc.hide();
            }
        }));
        *cc_cell.borrow_mut() = Some(Rc::clone(&click_catcher));

        // Automatically hide the click catcher when all popups become invisible (event-driven panel coordinator)
        let l_visible_ref = Rc::clone(&launcher);
        let q_visible_ref = Rc::clone(&quick_settings);
        let c_visible_ref = Rc::clone(&calendar);
        let r_visible_cell = Rc::clone(&r_dismiss_cell);
        let cc_visible_ref = Rc::clone(&click_catcher);

        let update_cc_visibility = move || {
            let any_visible = l_visible_ref.window.is_visible() 
                || q_visible_ref.window.is_visible() 
                || c_visible_ref.window.is_visible()
                || r_visible_cell.borrow().as_ref().map(|r| r.is_menu_visible()).unwrap_or(false);
            if !any_visible {
                cc_visible_ref.hide();
            }
        };

        let cb = Rc::new(update_cc_visibility);

        let cb_l = Rc::clone(&cb);
        launcher.window.connect_visible_notify(move |_| {
            cb_l();
        });

        let cb_q = Rc::clone(&cb);
        quick_settings.window.connect_visible_notify(move |_| {
            cb_q();
        });

        let cb_c = Rc::clone(&cb);
        calendar.window.connect_visible_notify(move |_| {
            cb_c();
        });

        // Wire Launcher toggle
        let l_toggle = Rc::clone(&launcher);
        let q_toggle_l = Rc::clone(&quick_settings);
        let c_toggle_l = Rc::clone(&calendar);
        let r_toggle_l = Rc::clone(&r_dismiss_cell);
        let cc_l = Rc::clone(&click_catcher);

        let left_section = LeftSection::new(move || {
            let is_open = l_toggle.window.is_visible();
            q_toggle_l.close();
            c_toggle_l.close();
            if let Some(ref r) = *r_toggle_l.borrow() {
                r.close();
            }
            if is_open {
                l_toggle.close();
                cc_l.hide();
            } else {
                cc_l.show();
                l_toggle.toggle();
            }
        });

        let center_section = CenterSection::new();

        // Wire Quick Settings & Calendar toggles
        let q_toggle = Rc::clone(&quick_settings);
        let l_toggle_q = Rc::clone(&launcher);
        let c_toggle_q = Rc::clone(&calendar);
        let r_toggle_q = Rc::clone(&r_dismiss_cell);
        let cc_q = Rc::clone(&click_catcher);

        let c_toggle = Rc::clone(&calendar);
        let l_toggle_c = Rc::clone(&launcher);
        let q_toggle_c = Rc::clone(&quick_settings);
        let r_toggle_c = Rc::clone(&r_dismiss_cell);
        let cc_c = Rc::clone(&click_catcher);

        let cc_show = Rc::clone(&click_catcher);
        let cb_r2 = Rc::clone(&cb);
        let right_section = Rc::new(RightSection::new(
            move || {
                let is_open = q_toggle.window.is_visible();
                l_toggle_q.close();
                c_toggle_q.close();
                if let Some(ref r) = *r_toggle_q.borrow() {
                    r.close();
                }
                if is_open {
                    q_toggle.close();
                    cc_q.hide();
                } else {
                    cc_q.show();
                    q_toggle.toggle();
                }
            },
            move || {
                let is_open = c_toggle.window.is_visible();
                l_toggle_c.close();
                q_toggle_c.close();
                if let Some(ref r) = *r_toggle_c.borrow() {
                    r.close();
                }
                if is_open {
                    c_toggle.close();
                    cc_c.hide();
                } else {
                    cc_c.show();
                    c_toggle.toggle();
                }
            },
            move || {
                cc_show.show();
            },
            move || {
                cb_r2();
            }
        ));
        *r_dismiss_cell.borrow_mut() = Some(Rc::clone(&right_section));

        // NetworkManager live event listener via Unix pipe (epoll 0.0% CPU idle)
        let (net_read_fd, net_write_fd) = create_event_pipe();
        NetworkService::listen_events(move || {
            notify_pipe(net_write_fd);
        });

        let qs_net = Rc::clone(&quick_settings);
        let right_net = Rc::clone(&right_section);
        glib::unix_fd_add_local(net_read_fd, glib::IOCondition::IN, move |fd, _| {
            drain_pipe(fd);
            right_net.update_network_state();
            if qs_net.window.is_visible() && qs_net.stack.visible_child_name().as_deref() == Some("wifi") {
                qs_net.wifi_page.sync_state();
            }
            GridSection::async_refresh(Rc::clone(&qs_net.grid));
            glib::ControlFlow::Continue
        });

        // Bluetooth live event listener via Unix pipe (epoll 0.0% CPU idle)
        let (bt_read_fd, bt_write_fd) = create_event_pipe();
        BluetoothService::listen_events(move || {
            notify_pipe(bt_write_fd);
        });

        let qs_bt = Rc::clone(&quick_settings);
        let right_bt = Rc::clone(&right_section);
        glib::unix_fd_add_local(bt_read_fd, glib::IOCondition::IN, move |fd, _| {
            drain_pipe(fd);
            right_bt.update_bluetooth_state();
            if qs_bt.window.is_visible() && qs_bt.stack.visible_child_name().as_deref() == Some("bt") {
                qs_bt.bt_page.sync_state();
            }
            GridSection::async_refresh(Rc::clone(&qs_bt.grid));
            glib::ControlFlow::Continue
        });

        // Main shelf container
        let main_box = GtkBox::new(Orientation::Horizontal, 0);
        main_box.add_css_class("cos-bar-container");

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
            launcher,
            click_catcher,
        }
    }

    pub fn show(&self) {
        self.window.set_visible(true);
        self.window.present();
        eprintln!("[bar] Bar window presented successfully");
    }
}
