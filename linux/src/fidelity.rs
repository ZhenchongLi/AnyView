use sha2::{Digest, Sha256};
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use crate::libreoffice_cli;
use crate::renderer::FidelityError;

pub const SUPPORTED_EXTENSIONS: &[&str] = &["docx", "docmod", "doct", "pptx", "ppt", "xlsx", "xls"];

pub fn is_supported(ext: &str) -> bool {
    SUPPORTED_EXTENSIONS.contains(&ext)
}

pub fn cached_pdf_path(source_path: &Path) -> Option<PathBuf> {
    let key = cache_key(source_path).ok()?;
    let candidate = cache_dir().join(format!("{key}.pdf"));
    candidate.exists().then_some(candidate)
}

pub fn prepare_pdf(source_path: &Path) -> Result<PathBuf, FidelityError> {
    if let Some(cached) = cached_pdf_path(source_path) {
        return Ok(cached);
    }

    if libreoffice_cli::find_soffice().is_none() {
        return Err(FidelityError::SofficeNotFound);
    }

    let ext = source_path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    let mut extract_dir_guard = None;
    let input_for_soffice = if ext == "docmod" || ext == "doct" {
        let extract_dir = tempfile::Builder::new()
            .prefix("anyview-docmod-")
            .tempdir()
            .map_err(|e| FidelityError::ConversionFailed(e.to_string()))?;
        let input = extract_source_docx(source_path, extract_dir.path())?;
        extract_dir_guard = Some(extract_dir);
        input
    } else {
        source_path.to_path_buf()
    };
    let _extract_dir_guard = extract_dir_guard;

    let output_dir = tempfile::Builder::new()
        .prefix("anyview-lo-output-")
        .tempdir()
        .map_err(|e| FidelityError::ConversionFailed(e.to_string()))?;

    let pdf_path =
        libreoffice_cli::convert_to_pdf(&input_for_soffice, output_dir.path()).map_err(|e| {
            if libreoffice_cli::find_soffice().is_none() {
                FidelityError::SofficeNotFound
            } else {
                FidelityError::ConversionFailed(e)
            }
        })?;

    store_pdf(&pdf_path, source_path).ok_or_else(|| {
        FidelityError::ConversionFailed("Failed to store converted PDF in cache.".to_string())
    })
}

fn extract_source_docx(zip_path: &Path, output_dir: &Path) -> Result<PathBuf, FidelityError> {
    let file = File::open(zip_path).map_err(|e| FidelityError::ConversionFailed(e.to_string()))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| FidelityError::ConversionFailed(e.to_string()))?;
    let mut source = archive
        .by_name("source.docx")
        .map_err(|_| FidelityError::NoSourceDocx)?;
    let target = output_dir.join("source.docx");
    let mut output =
        File::create(&target).map_err(|e| FidelityError::ConversionFailed(e.to_string()))?;
    io::copy(&mut source, &mut output)
        .map_err(|e| FidelityError::ConversionFailed(e.to_string()))?;
    Ok(target)
}

fn store_pdf(pdf_path: &Path, source_path: &Path) -> Option<PathBuf> {
    let key = cache_key(source_path).ok()?;
    let dir = cache_dir();
    std::fs::create_dir_all(&dir).ok()?;
    let target = dir.join(format!("{key}.pdf"));
    let _ = std::fs::remove_file(&target);
    std::fs::copy(pdf_path, &target).ok()?;
    Some(target)
}

fn cache_key(path: &Path) -> io::Result<String> {
    use std::os::unix::fs::MetadataExt;

    let metadata = std::fs::metadata(path)?;
    let path_hash = sha256_hex(path.to_string_lossy().as_bytes());
    let fingerprint = format!(
        "{}:{}:{}",
        metadata.size(),
        metadata.mtime(),
        metadata.mtime_nsec()
    );
    let file_hash = sha256_hex(fingerprint.as_bytes());
    Ok(format!("{}-{}", &path_hash[..12], &file_hash[..12]))
}

fn cache_dir() -> PathBuf {
    if let Some(path) = std::env::var_os("XDG_CACHE_HOME") {
        return PathBuf::from(path).join("anyview/fidelity");
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".cache/anyview/fidelity");
    }
    std::env::temp_dir().join("anyview/fidelity")
}

fn sha256_hex(bytes: &[u8]) -> String {
    let hash = Sha256::digest(bytes);
    hash.iter().map(|b| format!("{b:02x}")).collect()
}
