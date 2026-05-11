use gtk::prelude::*;
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use crate::renderer::{FindCompletion, Renderer};

#[derive(Clone)]
pub struct PdfRenderer {
    scrolled: gtk::ScrolledWindow,
    container: gtk::Box,
    document: Rc<RefCell<Option<poppler::Document>>>,
    areas: Rc<RefCell<Vec<gtk::DrawingArea>>>,
    zoom: Rc<RefCell<f64>>,
    find_state: Rc<RefCell<PdfFindState>>,
}

#[derive(Clone, Debug)]
struct PdfFindHit {
    page_index: i32,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
}

#[derive(Default)]
struct PdfFindState {
    query: String,
    matches: Vec<PdfFindHit>,
    current_index: Option<usize>,
}

impl PdfRenderer {
    pub const fn extensions() -> &'static [&'static str] {
        &["pdf"]
    }

    pub fn supports(ext: &str) -> bool {
        Self::extensions().contains(&ext)
    }

    pub fn new() -> Self {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 12);
        container.set_margin_top(12);
        container.set_margin_bottom(12);
        container.set_margin_start(12);
        container.set_margin_end(12);
        container.set_halign(gtk::Align::Center);

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Automatic)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .child(&container)
            .build();

        Self {
            scrolled,
            container,
            document: Rc::new(RefCell::new(None)),
            areas: Rc::new(RefCell::new(Vec::new())),
            zoom: Rc::new(RefCell::new(1.0)),
            find_state: Rc::new(RefCell::new(PdfFindState::default())),
        }
    }

    fn clear(&self) {
        while let Some(child) = self.container.first_child() {
            self.container.remove(&child);
        }
        self.areas.borrow_mut().clear();
    }

    fn show_error(&self, message: &str) {
        let label = gtk::Label::new(Some(message));
        label.set_wrap(true);
        label.set_margin_top(24);
        label.set_margin_bottom(24);
        self.container.append(&label);
    }

    fn rebuild_matches(&self, query: &str) {
        let mut matches = Vec::new();
        if let Some(document) = self.document.borrow().as_ref() {
            for page_index in 0..document.n_pages() {
                let Some(page) = document.page(page_index) else {
                    continue;
                };
                for rect in page.find_text_with_options(
                    query,
                    poppler::FindFlags::DEFAULT | poppler::FindFlags::MULTILINE,
                ) {
                    matches.push(PdfFindHit {
                        page_index,
                        x1: rect.x1(),
                        y1: rect.y1(),
                        x2: rect.x2(),
                        y2: rect.y2(),
                    });
                }
            }
        }

        let mut state = self.find_state.borrow_mut();
        state.query = query.to_string();
        state.matches = matches;
        state.current_index = None;
    }

    fn queue_find_redraw(&self) {
        for area in self.areas.borrow().iter() {
            area.queue_draw();
        }
    }

    fn scroll_to_current_match(&self) {
        let hit = {
            let state = self.find_state.borrow();
            let Some(index) = state.current_index else {
                return;
            };
            state.matches.get(index).cloned()
        };
        let Some(hit) = hit else {
            return;
        };
        let Some(area) = self.areas.borrow().get(hit.page_index as usize).cloned() else {
            return;
        };

        let container = self.container.clone();
        let scrolled = self.scrolled.clone();
        let zoom = *self.zoom.borrow();
        glib::idle_add_local_once(move || {
            let Some(bounds) = area.compute_bounds(&container) else {
                return;
            };
            let vadj = scrolled.vadjustment();
            let target = bounds.y() as f64 + hit.y1 * zoom - 48.0;
            let max = (vadj.upper() - vadj.page_size()).max(vadj.lower());
            vadj.set_value(target.clamp(vadj.lower(), max));

            let hadj = scrolled.hadjustment();
            let target_x = bounds.x() as f64 + hit.x1 * zoom - 48.0;
            let max_x = (hadj.upper() - hadj.page_size()).max(hadj.lower());
            hadj.set_value(target_x.clamp(hadj.lower(), max_x));
        });
    }
}

impl Renderer for PdfRenderer {
    fn widget(&self) -> gtk::Widget {
        self.scrolled.clone().upcast()
    }

    fn load(&self, path: &Path) {
        self.clear();
        self.find_state.replace(PdfFindState::default());

        let uri = match glib::filename_to_uri(path, None) {
            Ok(u) => u,
            Err(e) => {
                self.show_error(&format!("Failed to build file URI: {e}"));
                return;
            }
        };

        let document = match poppler::Document::from_file(&uri, None) {
            Ok(d) => d,
            Err(e) => {
                self.show_error(&format!("Failed to load PDF: {e}"));
                return;
            }
        };

        let n_pages = document.n_pages();
        self.document.replace(Some(document));

        for index in 0..n_pages {
            let area = gtk::DrawingArea::new();
            area.set_halign(gtk::Align::Center);

            // Initial size from the page at current zoom
            if let Some(page) = self.document.borrow().as_ref().and_then(|d| d.page(index)) {
                let (w, h) = page.size();
                let zoom = *self.zoom.borrow();
                area.set_content_width((w * zoom) as i32);
                area.set_content_height((h * zoom) as i32);
            }

            let document_ref = self.document.clone();
            let zoom_ref = self.zoom.clone();
            let find_state_ref = self.find_state.clone();
            let page_index = index;
            area.set_draw_func(move |_area, ctx, _width, _height| {
                let zoom = *zoom_ref.borrow();
                let doc_borrow = document_ref.borrow();
                let Some(doc) = doc_borrow.as_ref() else {
                    return;
                };
                let Some(page) = doc.page(page_index) else {
                    return;
                };

                // Paint a white page background so transparent PDFs aren't
                // rendered on whatever the window background is.
                let (pw, ph) = page.size();
                ctx.save().ok();
                ctx.scale(zoom, zoom);
                ctx.set_source_rgb(1.0, 1.0, 1.0);
                ctx.rectangle(0.0, 0.0, pw, ph);
                let _ = ctx.fill();
                page.render(ctx);
                let state = find_state_ref.borrow();
                for (hit_index, hit) in state.matches.iter().enumerate() {
                    if hit.page_index != page_index {
                        continue;
                    }
                    let is_current = state.current_index == Some(hit_index);
                    if is_current {
                        ctx.set_source_rgba(1.0, 0.55, 0.0, 0.45);
                    } else {
                        ctx.set_source_rgba(1.0, 0.88, 0.0, 0.30);
                    }
                    ctx.rectangle(
                        hit.x1,
                        hit.y1,
                        (hit.x2 - hit.x1).max(1.0),
                        (hit.y2 - hit.y1).max(1.0),
                    );
                    let _ = ctx.fill();
                }
                ctx.restore().ok();
            });

            self.container.append(&area);
            self.areas.borrow_mut().push(area);
        }

        if n_pages == 0 {
            self.show_error("PDF contains no pages.");
        }
    }

    fn set_zoom(&self, level: f64) {
        let level = level.max(0.1);
        *self.zoom.borrow_mut() = level;

        let doc_borrow = self.document.borrow();
        let Some(doc) = doc_borrow.as_ref() else {
            return;
        };

        for (idx, area) in self.areas.borrow().iter().enumerate() {
            if let Some(page) = doc.page(idx as i32) {
                let (w, h) = page.size();
                area.set_content_width((w * level) as i32);
                area.set_content_height((h * level) as i32);
            }
            area.queue_resize();
            area.queue_draw();
        }
    }

    fn supports_find(&self) -> bool {
        true
    }

    fn perform_find(&self, query: &str, forward: bool, completion: FindCompletion) {
        if query.trim().is_empty() {
            completion(false);
            return;
        }

        if self.find_state.borrow().query != query {
            self.rebuild_matches(query);
        }

        let found = {
            let mut state = self.find_state.borrow_mut();
            if state.matches.is_empty() {
                state.current_index = None;
                false
            } else {
                let len = state.matches.len();
                let next = match (state.current_index, forward) {
                    (Some(index), true) => (index + 1) % len,
                    (Some(0), false) | (None, false) => len - 1,
                    (Some(index), false) => index - 1,
                    (None, true) => 0,
                };
                state.current_index = Some(next);
                true
            }
        };

        self.queue_find_redraw();
        if found {
            self.scroll_to_current_match();
        }
        completion(found);
    }
}
