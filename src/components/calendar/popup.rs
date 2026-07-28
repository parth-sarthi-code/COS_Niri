use chrono::{Datelike, Local, NaiveDate};
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Box as GtkBox, Button, Grid, Label, Orientation};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::cell::RefCell;
use std::rc::Rc;

pub struct CalendarPopup {
    pub window: ApplicationWindow,
    current_date: Rc<RefCell<NaiveDate>>,
    month_label: Label,
    grid: Grid,
}

impl CalendarPopup {
    pub fn new(app: &Application) -> Self {
        let window = ApplicationWindow::new(app);

        // Configure Layer-Shell popup window
        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_namespace("cos-calendar");

        // Anchor to Bottom-Right floating above the bar shelf (aligned next to date pill)
        window.set_anchor(Edge::Bottom, true);
        window.set_anchor(Edge::Right, true);
        window.set_margin(Edge::Bottom, 56);
        window.set_margin(Edge::Right, 24);

        window.add_css_class("cal-popup-window");

        let container = GtkBox::new(Orientation::Vertical, 18);
        container.add_css_class("cal-popup-container");
        container.set_size_request(370, -1);

        let today = Local::now().date_naive();
        let current_date = Rc::new(RefCell::new(today));

        // 1. Header (Month Year Title | Prev, Next, Today)
        let header = GtkBox::new(Orientation::Horizontal, 8);
        header.add_css_class("cal-header");
        header.set_valign(gtk4::Align::Center);

        let month_label = Label::new(Some(&today.format("%B %Y").to_string()));
        month_label.add_css_class("cal-month-title");
        month_label.set_hexpand(true);
        month_label.set_halign(gtk4::Align::Start);
        header.append(&month_label);

        // Today pill button
        let today_btn = Button::with_label("Today");
        today_btn.add_css_class("cal-today-btn");

        // Prev month button (<)
        let prev_btn = Button::new();
        prev_btn.add_css_class("cal-nav-btn");
        let prev_icon = Label::new(Some("\u{e5cb}")); // chevron_left
        prev_icon.add_css_class("ms-icon");
        prev_icon.add_css_class("ms-icon-sm");
        prev_btn.set_child(Some(&prev_icon));

        // Next month button (>)
        let next_btn = Button::new();
        next_btn.add_css_class("cal-nav-btn");
        let next_icon = Label::new(Some("\u{e5cc}")); // chevron_right
        next_icon.add_css_class("ms-icon");
        next_icon.add_css_class("ms-icon-sm");
        next_btn.set_child(Some(&next_icon));

        let nav_box = GtkBox::new(Orientation::Horizontal, 4);
        nav_box.append(&today_btn);
        nav_box.append(&prev_btn);
        nav_box.append(&next_btn);
        header.append(&nav_box);

        container.append(&header);

        // 2. Day-of-Week Headers (Mon Tue Wed Thu Fri Sat Sun)
        let dow_box = Grid::new();
        dow_box.set_column_homogeneous(true);
        dow_box.add_css_class("cal-dow-grid");

        let days_of_week = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        for (i, day_str) in days_of_week.iter().enumerate() {
            let lbl = Label::new(Some(day_str));
            lbl.add_css_class("cal-dow-label");
            lbl.set_halign(gtk4::Align::Center);
            dow_box.attach(&lbl, i as i32, 0, 1, 1);
        }
        container.append(&dow_box);

        // 3. 7x6 Calendar Days Grid
        let grid = Grid::new();
        grid.set_column_homogeneous(true);
        grid.set_row_homogeneous(true);
        grid.set_row_spacing(4);
        grid.set_column_spacing(4);
        grid.add_css_class("cal-days-grid");
        container.append(&grid);

        window.set_child(Some(&container));

        let popup = Self {
            window,
            current_date: Rc::clone(&current_date),
            month_label: month_label.clone(),
            grid: grid.clone(),
        };

        // Render current month grid
        popup.render_grid();

        // Connect navigation handlers
        let pop_prev = popup.clone_ref();
        prev_btn.connect_clicked(move |_| {
            pop_prev.prev_month();
        });

        let pop_next = popup.clone_ref();
        next_btn.connect_clicked(move |_| {
            pop_next.next_month();
        });

        let pop_today = popup.clone_ref();
        today_btn.connect_clicked(move |_| {
            pop_today.go_today();
        });

        popup
    }

    fn clone_ref(&self) -> Self {
        Self {
            window: self.window.clone(),
            current_date: Rc::clone(&self.current_date),
            month_label: self.month_label.clone(),
            grid: self.grid.clone(),
        }
    }

    pub fn prev_month(&self) {
        let mut date = self.current_date.borrow_mut();
        let (y, m) = if date.month() == 1 {
            (date.year() - 1, 12)
        } else {
            (date.year(), date.month() - 1)
        };
        *date = NaiveDate::from_ymd_opt(y, m, 1).unwrap_or(*date);
        drop(date);
        self.render_grid();
    }

    pub fn next_month(&self) {
        let mut date = self.current_date.borrow_mut();
        let (y, m) = if date.month() == 12 {
            (date.year() + 1, 1)
        } else {
            (date.year(), date.month() + 1)
        };
        *date = NaiveDate::from_ymd_opt(y, m, 1).unwrap_or(*date);
        drop(date);
        self.render_grid();
    }

    pub fn go_today(&self) {
        let mut date = self.current_date.borrow_mut();
        *date = Local::now().date_naive();
        drop(date);
        self.render_grid();
    }

    pub fn render_grid(&self) {
        // Clear existing grid children
        while let Some(child) = self.grid.first_child() {
            self.grid.remove(&child);
        }

        let curr = *self.current_date.borrow();
        self.month_label.set_text(&curr.format("%B %Y").to_string());

        let first_of_month = NaiveDate::from_ymd_opt(curr.year(), curr.month(), 1).unwrap();
        // Iso weekday: Mon=1..Sun=7
        let start_weekday = first_of_month.weekday().number_from_monday() as i32; // 1..7

        // Days in current month
        let days_in_month = if curr.month() == 12 {
            NaiveDate::from_ymd_opt(curr.year() + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(curr.year(), curr.month() + 1, 1)
        }
        .unwrap()
        .signed_duration_since(first_of_month)
        .num_days() as i32;

        let today = Local::now().date_naive();

        let mut day_counter = 1;
        for row in 0..6 {
            for col in 0..7 {
                let cell_num = row * 7 + col + 1; // 1-based index
                let is_current_month = cell_num >= start_weekday && day_counter <= days_in_month;

                let day_btn = Button::new();
                day_btn.add_css_class("cal-day-cell");

                if is_current_month {
                    let day_val = day_counter;
                    let is_today = curr.year() == today.year()
                        && curr.month() == today.month()
                        && day_val == today.day() as i32;

                    day_btn.set_label(&day_val.to_string());
                    if is_today {
                        day_btn.add_css_class("cal-today");
                    } else {
                        day_btn.add_css_class("cal-active-month");
                    }
                    day_counter += 1;
                } else {
                    day_btn.add_css_class("cal-other-month");
                    day_btn.set_sensitive(false);
                }

                self.grid.attach(&day_btn, col, row, 1, 1);
            }
        }
    }

    pub fn toggle(&self) {
        if self.window.is_visible() {
            crate::services::animation::slide_down_close(&self.window, 56);
        } else {
            self.go_today();
            crate::services::animation::slide_up_open(&self.window, 56);
        }
    }
}
