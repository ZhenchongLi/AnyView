use std::path::{Path, PathBuf};
use std::process::Command;

pub fn find_soffice() -> Option<PathBuf> {
    for candidate in [
        "/usr/bin/soffice",
        "/usr/local/bin/soffice",
        "/usr/bin/libreoffice",
        "/usr/local/bin/libreoffice",
        "/snap/bin/libreoffice",
        "/var/lib/flatpak/exports/bin/org.libreoffice.LibreOffice",
    ] {
        let path = PathBuf::from(candidate);
        if is_executable(&path) {
            return Some(path);
        }
    }

    resolve_via_path("soffice").or_else(|| resolve_via_path("libreoffice"))
}

pub fn convert_to_pdf(input_path: &Path, output_dir: &Path) -> Result<PathBuf, String> {
    let Some(soffice) = find_soffice() else {
        return Err("LibreOffice (soffice) not found.".to_string());
    };

    std::fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output directory: {e}"))?;

    let profile_dir = tempfile::Builder::new()
        .prefix("anyview-lo-profile-")
        .tempdir()
        .map_err(|e| format!("Failed to create LibreOffice profile: {e}"))?;
    let profile_uri = format!("file://{}", profile_dir.path().to_string_lossy());

    let output = Command::new(&soffice)
        .arg("--headless")
        .arg("--convert-to")
        .arg("pdf")
        .arg("--outdir")
        .arg(output_dir)
        .arg(format!("-env:UserInstallation={profile_uri}"))
        .arg(input_path)
        .output()
        .map_err(|e| format!("Failed to run LibreOffice: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let code = output
            .status
            .code()
            .map(|c| c.to_string())
            .unwrap_or_else(|| "signal".to_string());
        return Err(format!(
            "LibreOffice exited with {code}\nstderr: {}\nstdout: {}",
            stderr.trim(),
            stdout.trim()
        ));
    }

    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let pdf_path = output_dir.join(format!("{stem}.pdf"));
    if pdf_path.exists() {
        Ok(pdf_path)
    } else {
        Err("LibreOffice produced no PDF output.".to_string())
    }
}

fn resolve_via_path(name: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if is_executable(&candidate) {
            return Some(candidate);
        }
    }
    None
}

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|m| m.is_file() && (m.permissions().mode() & 0o111) != 0)
        .unwrap_or(false)
}
