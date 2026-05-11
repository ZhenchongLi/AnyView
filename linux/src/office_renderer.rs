use gtk::prelude::*;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

use crate::pdf_renderer::PdfRenderer;
use crate::renderer::{FidelityError, FindCompletion, Renderer};

pub struct OfficeRenderer {
    stack: gtk::Stack,
    pdf: PdfRenderer,
    spinner: gtk::Spinner,
    label: gtk::Label,
    current_path: Rc<RefCell<Option<PathBuf>>>,
    conversion_token: Rc<RefCell<u64>>,
}

impl OfficeRenderer {
    pub const fn extensions() -> &'static [&'static str] {
        &["pptx", "ppt"]
    }

    pub fn supports(ext: &str) -> bool {
        Self::extensions().contains(&ext)
    }

    pub fn new() -> Self {
        let pdf = PdfRenderer::new();
        let pdf_widget = pdf.widget();

        let spinner = gtk::Spinner::new();
        let label = gtk::Label::new(None);
        label.set_wrap(true);
        label.set_justify(gtk::Justification::Center);
        label.set_margin_start(24);
        label.set_margin_end(24);

        let status_box = gtk::Box::new(gtk::Orientation::Vertical, 12);
        status_box.set_halign(gtk::Align::Center);
        status_box.set_valign(gtk::Align::Center);
        status_box.append(&spinner);
        status_box.append(&label);

        let stack = gtk::Stack::new();
        stack.set_hexpand(true);
        stack.set_vexpand(true);
        stack.add_named(&pdf_widget, Some("pdf"));
        stack.add_named(&status_box, Some("status"));
        stack.set_visible_child_name("status");

        Self {
            stack,
            pdf,
            spinner,
            label,
            current_path: Rc::new(RefCell::new(None)),
            conversion_token: Rc::new(RefCell::new(0)),
        }
    }

    fn show_status(&self, message: &str, spinning: bool) {
        self.label.set_text(message);
        if spinning {
            self.spinner.start();
        } else {
            self.spinner.stop();
        }
        self.stack.set_visible_child_name("status");
    }
}

impl Renderer for OfficeRenderer {
    fn widget(&self) -> gtk::Widget {
        self.stack.clone().upcast()
    }

    fn load(&self, path: &Path) {
        *self.current_path.borrow_mut() = Some(path.to_path_buf());

        if crate::libreoffice_cli::find_soffice().is_none() {
            self.show_status(&FidelityError::SofficeNotFound.to_string(), false);
            return;
        }

        if let Some(cached) = crate::fidelity::cached_pdf_path(path) {
            self.pdf.load(&cached);
            self.spinner.stop();
            self.stack.set_visible_child_name("pdf");
            return;
        }

        let token = {
            let mut token = self.conversion_token.borrow_mut();
            *token += 1;
            *token
        };
        self.show_status("Generating presentation preview...", true);

        let source_path = path.to_path_buf();
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = sender.send(crate::fidelity::prepare_pdf(&source_path));
        });

        let receiver = Rc::new(RefCell::new(receiver));
        let stack = self.stack.clone();
        let spinner = self.spinner.clone();
        let label = self.label.clone();
        let pdf = self.pdf.clone();
        let token_ref = self.conversion_token.clone();

        glib::timeout_add_local(Duration::from_millis(100), move || {
            match receiver.borrow().try_recv() {
                Ok(result) => {
                    if *token_ref.borrow() == token {
                        spinner.stop();
                        match result {
                            Ok(pdf_path) => {
                                pdf.load(&pdf_path);
                                stack.set_visible_child_name("pdf");
                            }
                            Err(err) => {
                                label.set_text(&err.to_string());
                                stack.set_visible_child_name("status");
                            }
                        }
                    }
                    glib::ControlFlow::Break
                }
                Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(mpsc::TryRecvError::Disconnected) => {
                    if *token_ref.borrow() == token {
                        spinner.stop();
                        label.set_text("LibreOffice conversion worker stopped unexpectedly.");
                        stack.set_visible_child_name("status");
                    }
                    glib::ControlFlow::Break
                }
            }
        });
    }

    fn set_zoom(&self, level: f64) {
        self.pdf.set_zoom(level);
    }

    fn supports_find(&self) -> bool {
        self.stack
            .visible_child_name()
            .map(|name| name.as_str() == "pdf")
            .unwrap_or(false)
    }

    fn perform_find(&self, query: &str, forward: bool, completion: FindCompletion) {
        if self.supports_find() {
            self.pdf.perform_find(query, forward, completion);
        } else {
            completion(false);
        }
    }
}
