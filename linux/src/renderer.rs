use std::path::Path;

use crate::image_renderer::ImageRenderer;
use crate::media_renderer::MediaRenderer;
use crate::office_renderer::OfficeRenderer;
use crate::pdf_renderer::PdfRenderer;
use crate::web_renderer::WebRenderer;

#[derive(Clone, Debug)]
pub enum FidelityError {
    SofficeNotFound,
    NoSourceDocx,
    ConversionFailed(String),
    UnsupportedExtension,
}

impl std::fmt::Display for FidelityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FidelityError::SofficeNotFound => write!(
                f,
                "LibreOffice (soffice) not found. Install LibreOffice to enable fidelity preview."
            ),
            FidelityError::NoSourceDocx => write!(f, "No source.docx found inside the package."),
            FidelityError::ConversionFailed(message) => {
                write!(f, "LibreOffice conversion failed: {message}")
            }
            FidelityError::UnsupportedExtension => {
                write!(f, "Fidelity mode does not support this file type.")
            }
        }
    }
}

pub type FindCompletion = Box<dyn Fn(bool) + 'static>;
pub type FidelityCompletion = Box<dyn Fn(Result<(), FidelityError>) + 'static>;

pub trait Renderer {
    fn widget(&self) -> gtk::Widget;
    fn load(&self, path: &Path);
    fn set_zoom(&self, _level: f64) {}
    fn supports_find(&self) -> bool {
        false
    }
    fn perform_find(&self, _query: &str, _forward: bool, completion: FindCompletion) {
        completion(false);
    }
    fn supports_fidelity(&self) -> bool {
        false
    }
    fn fidelity_enabled(&self) -> bool {
        false
    }
    fn set_fidelity(&self, _enabled: bool, completion: FidelityCompletion) {
        completion(Err(FidelityError::UnsupportedExtension));
    }
}

pub struct RendererFactory;

impl RendererFactory {
    pub fn renderer_for(ext: &str) -> Box<dyn Renderer> {
        let ext = ext.to_ascii_lowercase();
        if PdfRenderer::supports(&ext) {
            return Box::new(PdfRenderer::new());
        }
        if ImageRenderer::supports(&ext) {
            return Box::new(ImageRenderer::new());
        }
        if MediaRenderer::supports(&ext) {
            return Box::new(MediaRenderer::new());
        }
        if OfficeRenderer::supports(&ext) {
            return Box::new(OfficeRenderer::new());
        }
        Box::new(WebRenderer::new())
    }

    pub fn all_supported_extensions() -> Vec<&'static str> {
        let mut exts: Vec<&'static str> = Vec::new();
        exts.extend(PdfRenderer::extensions());
        exts.extend(ImageRenderer::extensions());
        exts.extend(MediaRenderer::extensions());
        exts.extend(OfficeRenderer::extensions());
        exts.extend(WebRenderer::extensions());
        exts
    }

    pub fn is_supported(ext: &str) -> bool {
        let ext = ext.to_ascii_lowercase();
        Self::all_supported_extensions().iter().any(|e| *e == ext)
    }
}
