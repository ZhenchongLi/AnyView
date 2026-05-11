use gtk::prelude::*;

pub struct FindBar {
    root: gtk::Box,
    entry: gtk::SearchEntry,
    status: gtk::Label,
    prev_button: gtk::Button,
    next_button: gtk::Button,
}

impl FindBar {
    pub fn new<F, C>(on_search: F, on_close: C) -> Self
    where
        F: Fn(String, bool) + Clone + 'static,
        C: Fn() + Clone + 'static,
    {
        let root = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        root.set_margin_top(4);
        root.set_margin_bottom(4);
        root.set_margin_start(12);
        root.set_margin_end(12);
        root.set_valign(gtk::Align::Center);
        root.add_css_class("toolbar");

        let entry = gtk::SearchEntry::new();
        entry.set_placeholder_text(Some("Find"));
        entry.set_width_chars(28);
        entry.set_hexpand(false);

        let status = gtk::Label::new(None);
        status.set_xalign(0.0);
        status.set_hexpand(true);
        status.add_css_class("dim-label");

        let prev_button = gtk::Button::from_icon_name("go-up-symbolic");
        prev_button.set_tooltip_text(Some("Previous (Shift+Enter)"));
        let next_button = gtk::Button::from_icon_name("go-down-symbolic");
        next_button.set_tooltip_text(Some("Next (Enter)"));
        let close_button = gtk::Button::from_icon_name("window-close-symbolic");
        close_button.set_tooltip_text(Some("Close (Esc)"));

        let nav_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        nav_box.add_css_class("linked");
        nav_box.append(&prev_button);
        nav_box.append(&next_button);

        root.append(&entry);
        root.append(&status);
        root.append(&nav_box);
        root.append(&close_button);
        root.set_visible(false);

        {
            let on_search = on_search.clone();
            entry.connect_activate(move |entry| {
                on_search(entry.text().to_string(), true);
            });
        }
        {
            let entry = entry.clone();
            let on_search = on_search.clone();
            prev_button.connect_clicked(move |_| {
                on_search(entry.text().to_string(), false);
            });
        }
        {
            let entry = entry.clone();
            let on_search = on_search.clone();
            next_button.connect_clicked(move |_| {
                on_search(entry.text().to_string(), true);
            });
        }
        {
            let on_close = on_close.clone();
            close_button.connect_clicked(move |_| on_close());
        }

        let key_controller = gtk::EventControllerKey::new();
        {
            let entry = entry.clone();
            let on_search = on_search.clone();
            let on_close = on_close.clone();
            key_controller.connect_key_pressed(move |_, key, _, state| match key {
                gdk::Key::Escape => {
                    on_close();
                    glib::Propagation::Stop
                }
                gdk::Key::Return | gdk::Key::KP_Enter => {
                    let backward = state.contains(gdk::ModifierType::SHIFT_MASK);
                    on_search(entry.text().to_string(), !backward);
                    glib::Propagation::Stop
                }
                _ => glib::Propagation::Proceed,
            });
        }
        entry.add_controller(key_controller);

        Self {
            root,
            entry,
            status,
            prev_button,
            next_button,
        }
    }

    pub fn widget(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }

    pub fn show(&self) {
        self.root.set_visible(true);
        self.entry.grab_focus();
        self.entry.select_region(0, -1);
    }

    pub fn hide(&self) {
        self.root.set_visible(false);
        self.status.set_text("");
    }

    pub fn is_visible(&self) -> bool {
        self.root.is_visible()
    }

    pub fn query(&self) -> String {
        self.entry.text().to_string()
    }

    pub fn set_status(&self, text: &str, is_error: bool) {
        self.status.set_text(text);
        if is_error {
            self.status.add_css_class("error");
        } else {
            self.status.remove_css_class("error");
        }
    }

    pub fn set_supported(&self, supported: bool) {
        self.entry.set_sensitive(supported);
        self.prev_button.set_sensitive(supported);
        self.next_button.set_sensitive(supported);
        if supported {
            self.set_status("", false);
        } else {
            self.set_status("Find is not available for this file.", true);
        }
    }
}
