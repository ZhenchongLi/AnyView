use gtk::prelude::*;
use std::path::Path;

use crate::renderer::Renderer;

pub struct ImageRenderer {
    scrolled: gtk::ScrolledWindow,
    picture: gtk::Picture,
}

impl ImageRenderer {
    pub const fn extensions() -> &'static [&'static str] {
        &[
            "png", "jpg", "jpeg", "gif", "webp", "tiff", "tif", "bmp", "ico", "svg", "heic", "heif",
        ]
    }

    pub fn supports(ext: &str) -> bool {
        Self::extensions().contains(&ext)
    }

    pub fn new() -> Self {
        let picture = gtk::Picture::new();
        picture.set_hexpand(true);
        picture.set_vexpand(true);
        picture.set_can_shrink(true);

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Automatic)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .child(&picture)
            .build();

        Self { scrolled, picture }
    }
}

impl Renderer for ImageRenderer {
    fn widget(&self) -> gtk::Widget {
        self.scrolled.clone().upcast()
    }

    fn load(&self, path: &Path) {
        self.picture.set_filename(Some(path));
    }
}
