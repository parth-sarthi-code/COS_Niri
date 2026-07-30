use crate::niri_ipc::{NiriIpcClient, NiriWorkspace};
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Label, Orientation, Separator, EventControllerScroll, EventControllerScrollFlags};
use gtk4::glib::Propagation;
use niri_ipc::Event;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

struct WorkspaceState {
    workspace_box: GtkBox,
    pills: HashMap<u64, Button>,
    current_order: Vec<u64>,
    active_id: Option<u64>,
}

thread_local! {
    static STATE: RefCell<Option<Rc<RefCell<WorkspaceState>>>> = const { RefCell::new(None) };
}

pub struct LeftSection {
    pub container: GtkBox,
    #[allow(dead_code)]
    pub workspace_box: GtkBox,
}

impl LeftSection {
    pub fn new<F>(on_toggle_launcher: F) -> Self
    where
        F: Fn() + 'static,
    {
        let container = GtkBox::new(Orientation::Horizontal, 6);
        container.add_css_class("left-section");
        container.set_valign(gtk4::Align::Center);

        // Launcher button
        let launcher_btn = Button::new();
        launcher_btn.add_css_class("icon-btn-circle");
        launcher_btn.set_tooltip_text(Some("Launcher"));
        launcher_btn.set_valign(gtk4::Align::Center);

        let launcher_wrap = GtkBox::new(Orientation::Horizontal, 0);
        launcher_wrap.add_css_class("launcher-bubble");
        launcher_wrap.set_halign(gtk4::Align::Center);
        launcher_wrap.set_valign(gtk4::Align::Center);
        launcher_wrap.set_size_request(36, 36);

        let launcher_icon = Label::new(Some("\u{e5c3}"));
        launcher_icon.add_css_class("ms-icon");
        launcher_icon.set_halign(gtk4::Align::Center);
        launcher_icon.set_valign(gtk4::Align::Center);
        launcher_icon.set_hexpand(true);
        launcher_icon.set_vexpand(true);
        launcher_wrap.append(&launcher_icon);
        launcher_btn.set_child(Some(&launcher_wrap));
        container.append(&launcher_btn);

        launcher_btn.connect_clicked(move |_| {
            on_toggle_launcher();
        });

        // Vertical separator
        let sep = Separator::new(Orientation::Vertical);
        sep.add_css_class("shelf-sep");
        sep.set_valign(gtk4::Align::Center);
        container.append(&sep);

        // Workspace pills container
        let workspace_box = GtkBox::new(Orientation::Horizontal, 4);
        workspace_box.add_css_class("workspace-container");
        workspace_box.set_valign(gtk4::Align::Center);

        container.append(&workspace_box);

        let state = Rc::new(RefCell::new(WorkspaceState {
            workspace_box: workspace_box.clone(),
            pills: HashMap::new(),
            current_order: Vec::new(),
            active_id: None,
        }));

        STATE.with(|cell| {
            *cell.borrow_mut() = Some(Rc::clone(&state));
        });

        // Register mouse scroll controller on the workspace pills container
        let scroll = EventControllerScroll::new(EventControllerScrollFlags::VERTICAL);
        let state_clone = Rc::clone(&state);
        let accumulated = Rc::new(std::cell::Cell::new(0.0f64));
        let last_trigger = Rc::new(std::cell::Cell::new(std::time::Instant::now() - std::time::Duration::from_millis(500)));

        scroll.connect_scroll(move |_controller, _dx, dy| {
            let now = std::time::Instant::now();
            if now.duration_since(last_trigger.get()) < std::time::Duration::from_millis(150) {
                return Propagation::Stop;
            }

            let mut acc = accumulated.get();
            if (acc > 0.0 && dy < 0.0) || (acc < 0.0 && dy > 0.0) {
                acc = 0.0;
            }
            acc += dy;

            let mut triggered = false;
            let mut steps = 0i32;

            if acc.abs() >= 1.0 {
                steps = acc.signum() as i32;
                acc = 0.0;
                triggered = true;
            }

            accumulated.set(acc);

            if triggered {
                last_trigger.set(now);
                let st = state_clone.borrow();
                if let Some(active_id) = st.active_id {
                    if let Some(pos) = st.current_order.iter().position(|&id| id == active_id) {
                        let target_id = if steps < 0 {
                            // Scroll Up -> Focus previous workspace
                            if pos > 0 {
                                Some(st.current_order[pos - 1])
                            } else {
                                None
                            }
                        } else if steps > 0 {
                            // Scroll Down -> Focus next workspace
                            if pos + 1 < st.current_order.len() {
                                Some(st.current_order[pos + 1])
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        if let Some(tid) = target_id {
                            NiriIpcClient::focus_workspace_id(tid);
                        }
                    }
                }
            }
            Propagation::Stop
        });
        workspace_box.add_controller(scroll);

        // Initialize state from Niri
        if let Ok(workspaces) = NiriIpcClient::get_workspaces() {
            Self::update_workspaces(&state, workspaces);
        } else {
            Self::init_fallback(&state);
        }

        // Start background IPC event stream listener
        Self::start_workspace_listener();

        Self { container, workspace_box }
    }

    /// Optimized workspace update with in-place CSS mutations (0 child re-parenting if order is unchanged)
    fn update_workspaces(state_rc: &Rc<RefCell<WorkspaceState>>, mut workspaces: Vec<NiriWorkspace>) {
        workspaces.sort_by_key(|ws| ws.idx);
        let mut state = state_rc.borrow_mut();

        // Find active workspace ID
        let active_id = workspaces.iter().find(|ws| ws.is_active || ws.is_focused).map(|ws| ws.id);
        state.active_id = active_id;
        let new_order: Vec<u64> = workspaces.iter().map(|ws| ws.id).collect();

        // Check if workspace set or order changed
        let order_changed = state.current_order != new_order;

        // Remove pills for workspaces that no longer exist
        let old_ids: Vec<u64> = state.pills.keys().cloned().collect();
        for old_id in old_ids {
            if !new_order.contains(&old_id) {
                if let Some(btn) = state.pills.remove(&old_id) {
                    state.workspace_box.remove(&btn);
                }
            }
        }

        // Create or update pill properties
        for ws in &workspaces {
            let label_text = ws.name.clone().unwrap_or_else(|| ws.idx.to_string());
            let is_active = Some(ws.id) == active_id;

            if let Some(btn) = state.pills.get(&ws.id) {
                btn.set_label(&label_text);
                if is_active {
                    btn.add_css_class("active");
                } else {
                    btn.remove_css_class("active");
                }
            } else {
                let btn = Button::with_label(&label_text);
                btn.add_css_class("ws-pill");
                if is_active {
                    btn.add_css_class("active");
                }

                let ws_id = ws.id;
                btn.connect_clicked(move |clicked_btn| {
                    // Optimistic UI response
                    STATE.with(|cell| {
                        if let Some(st_rc) = cell.borrow().as_ref() {
                            let st = st_rc.borrow();
                            for pill in st.pills.values() {
                                pill.remove_css_class("active");
                            }
                        }
                    });
                    clicked_btn.add_css_class("active");

                    // Trigger Niri focus in background
                    NiriIpcClient::focus_workspace_id(ws_id);
                });

                state.pills.insert(ws.id, btn.clone());
            }
        }

        // Only re-append children if the workspace sequence or set actually changed
        if order_changed {
            while let Some(child) = state.workspace_box.first_child() {
                state.workspace_box.remove(&child);
            }
            for ws in &workspaces {
                if let Some(btn) = state.pills.get(&ws.id) {
                    state.workspace_box.append(btn);
                }
            }
            state.current_order = new_order;
        }
    }

    /// Update active workspace pill in-place on WorkspaceActivated event
    fn set_active_workspace(state_rc: &Rc<RefCell<WorkspaceState>>, active_id: u64) {
        let mut state = state_rc.borrow_mut();
        state.active_id = Some(active_id);
        for (&id, btn) in &state.pills {
            if id == active_id {
                btn.add_css_class("active");
            } else {
                btn.remove_css_class("active");
            }
        }
    }

    fn init_fallback(state_rc: &Rc<RefCell<WorkspaceState>>) {
        let mut state = state_rc.borrow_mut();
        let mut fallback_order = Vec::new();
        for i in 1..=3 {
            let btn = Button::with_label(&i.to_string());
            btn.add_css_class("ws-pill");
            if i == 1 {
                btn.add_css_class("active");
            }
            let idx = i as u8;
            btn.connect_clicked(move |_| {
                NiriIpcClient::focus_workspace_index(idx);
            });
            state.workspace_box.append(&btn);
            state.pills.insert(i as u64, btn);
            fallback_order.push(i as u64);
        }
        state.current_order = fallback_order;
    }

    /// Background listener loop for Niri IPC events
    fn start_workspace_listener() {
        // Debounce pipe for window dock updates — coalesces N rapid events into 1 IPC call
        let (dock_rfd, dock_wfd) = crate::bar::create_event_pipe();

        glib::unix_fd_add_local(dock_rfd, glib::IOCondition::IN, move |fd, _| {
            crate::bar::drain_pipe(fd);
            if let Ok(windows) = NiriIpcClient::get_windows() {
                crate::components::center::CenterSection::update_dock(&windows);
            }
            glib::ControlFlow::Continue
        });

        std::thread::spawn(move || {
            let _ = NiriIpcClient::listen_events(move |event| match event {
                Event::WorkspacesChanged { workspaces } => {
                    glib::idle_add_once(move || {
                        STATE.with(|cell| {
                            if let Some(st) = cell.borrow().as_ref() {
                                Self::update_workspaces(st, workspaces);
                            }
                        });
                    });
                }
                Event::WorkspaceActivated { id, focused: _ } => {
                    glib::idle_add_once(move || {
                        STATE.with(|cell| {
                            if let Some(st) = cell.borrow().as_ref() {
                                Self::set_active_workspace(st, id);
                            }
                        });
                    });
                }
                Event::WindowsChanged { windows } => {
                    glib::idle_add_once(move || {
                        crate::components::center::CenterSection::update_dock(&windows);
                    });
                }
                Event::WindowOpenedOrChanged { .. }
                | Event::WindowClosed { .. }
                | Event::WindowFocusChanged { .. } => {
                    // Signal the debounce pipe — GLib main loop coalesces and makes 1 IPC call
                    crate::bar::notify_pipe(dock_wfd);
                }
                _ => {}
            });
        });
    }
}
