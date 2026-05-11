use gtk::prelude::*;
use std::cell::RefCell;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;
use tempfile::TempDir;
use webkit::prelude::*;

use pulldown_cmark::{html as cmark_html, Options, Parser};

use crate::pdf_renderer::PdfRenderer;
use crate::renderer::{FidelityCompletion, FidelityError, FindCompletion, Renderer};

const HIGHLIGHT_JS: &str = include_str!("../resources/highlight.min.js");
const HLJS_LATEX_JS: &str = include_str!("../resources/hljs-latex.js");
const MERMAID_JS: &str = include_str!("../resources/mermaid.min.js");
const JSZIP_JS: &str = include_str!("../resources/jszip.min.js");
const DOCX_PREVIEW_JS: &str = include_str!("../resources/docx-preview.js");
const XLSX_JS: &str = include_str!("../resources/xlsx.full.min.js");

enum IWorkPreview {
    Pdf(PathBuf, Option<TempDir>),
    Image(PathBuf, Option<TempDir>),
}

pub struct WebRenderer {
    stack: gtk::Stack,
    webview: webkit::WebView,
    fidelity_pdf: PdfRenderer,
    status_spinner: gtk::Spinner,
    status_label: gtk::Label,
    current_path: Rc<RefCell<Option<PathBuf>>>,
    fidelity_enabled: Rc<RefCell<bool>>,
    conversion_token: Rc<RefCell<u64>>,
    pending_find: Rc<RefCell<Option<FindCompletion>>>,
    last_find_query: Rc<RefCell<String>>,
    temp_dirs: Rc<RefCell<Vec<TempDir>>>,
}

impl WebRenderer {
    pub const fn extensions() -> &'static [&'static str] {
        &[
            // Word docs (rendered via docmod CLI)
            "docx",
            "docmod",
            "doct",
            // iWork packages (QuickLook preview PDF when present)
            "key",
            "numbers",
            "pages",
            // Spreadsheets
            "xlsx",
            "xls",
            // HTML
            "html",
            "htm",
            // Markdown
            "md",
            "markdown",
            // LaTeX (compiled via tectonic)
            "tex",
            // LaTeX auxiliary (syntax highlight only)
            "sty",
            "cls",
            "bib",
            "bbl",
            // Subtitles
            "srt",
            "vtt",
            "ass",
            "ssa",
            "sub",
            "sbv",
            // Video (native WebKit)
            "mp4",
            "mov",
            "m4v",
            "webm",
            "m2ts",
            "ts",
            "3gp",
            // Video (transcoded via ffmpeg)
            "mkv",
            "avi",
            "flv",
            "wmv",
            "ogv",
            "rmvb",
            "rm",
            "asf",
            "vob",
            "divx",
            "f4v",
            // 3D models
            "stl",
            "obj",
            "usdz",
            "usd",
            "dae",
            // Fonts
            "ttf",
            "otf",
            "ttc",
            // Communication
            "vcf",
            "ics",
            // Code — languages
            "swift",
            "cs",
            "py",
            "js",
            "ts",
            "tsx",
            "jsx",
            "go",
            "rs",
            "rb",
            "java",
            "kt",
            "scala",
            "c",
            "h",
            "cpp",
            "hpp",
            "m",
            "mm",
            "lua",
            "r",
            "pl",
            "php",
            "dart",
            "zig",
            "nim",
            "ex",
            "exs",
            "erl",
            "hs",
            "ml",
            "fs",
            "v",
            "sv",
            "vhdl",
            "asm",
            "s",
            "sql",
            // Shell
            "sh",
            "bash",
            "zsh",
            "fish",
            "bat",
            "ps1",
            "cmd",
            // Data / config
            "xml",
            "json",
            "yaml",
            "yml",
            "toml",
            "ini",
            "cfg",
            "conf",
            "csv",
            "tsv",
            "plist",
            "graphql",
            "proto",
            // Web / styles
            "css",
            "scss",
            "sass",
            "less",
            // Docs / text
            "rst",
            "txt",
            "log",
            "diff",
            "patch",
            // Config files
            "env",
            "editorconfig",
            "gitignore",
            "gitattributes",
            "dockerignore",
            "makefile",
            "cmake",
            "gradle",
            "sln",
            "csproj",
            "xcodeproj",
            // Other
            "lock",
            "sum",
            "mod",
        ]
    }

    pub fn new() -> Self {
        let webview = webkit::WebView::new();
        let fidelity_pdf = PdfRenderer::new();
        let pdf_widget = fidelity_pdf.widget();

        let status_spinner = gtk::Spinner::new();
        let status_label = gtk::Label::new(None);
        status_label.set_wrap(true);
        status_label.set_justify(gtk::Justification::Center);
        status_label.set_margin_start(24);
        status_label.set_margin_end(24);
        let status_box = gtk::Box::new(gtk::Orientation::Vertical, 12);
        status_box.set_halign(gtk::Align::Center);
        status_box.set_valign(gtk::Align::Center);
        status_box.append(&status_spinner);
        status_box.append(&status_label);

        let stack = gtk::Stack::new();
        stack.set_hexpand(true);
        stack.set_vexpand(true);
        stack.add_named(&webview, Some("web"));
        stack.add_named(&pdf_widget, Some("pdf"));
        stack.add_named(&status_box, Some("status"));
        stack.set_visible_child_name("web");

        let pending_find: Rc<RefCell<Option<FindCompletion>>> = Rc::new(RefCell::new(None));
        if let Some(controller) = webview.find_controller() {
            let pending = pending_find.clone();
            controller.connect_found_text(move |_, count| {
                if let Some(completion) = pending.borrow_mut().take() {
                    completion(count > 0);
                }
            });
            let pending = pending_find.clone();
            controller.connect_failed_to_find_text(move |_| {
                if let Some(completion) = pending.borrow_mut().take() {
                    completion(false);
                }
            });
        }

        Self {
            stack,
            webview,
            fidelity_pdf,
            status_spinner,
            status_label,
            current_path: Rc::new(RefCell::new(None)),
            fidelity_enabled: Rc::new(RefCell::new(false)),
            conversion_token: Rc::new(RefCell::new(0)),
            pending_find,
            last_find_query: Rc::new(RefCell::new(String::new())),
            temp_dirs: Rc::new(RefCell::new(Vec::new())),
        }
    }

    fn ext_lower(path: &Path) -> String {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default()
    }

    fn file_uri(path: &Path) -> String {
        glib::filename_to_uri(path, None)
            .map(|g| g.to_string())
            .unwrap_or_else(|_| format!("file://{}", path.to_string_lossy()))
    }

    fn read_text_lossy(path: &Path) -> Result<String, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("Failed to read file: {e}"))?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    fn clear_temp_dirs(&self) {
        self.temp_dirs.borrow_mut().clear();
    }

    fn file_size_label(path: &Path) -> String {
        let Ok(metadata) = std::fs::metadata(path) else {
            return "-".to_string();
        };
        let size = metadata.len() as f64;
        if size >= 1024.0 * 1024.0 {
            format!("{:.1} MB", size / 1024.0 / 1024.0)
        } else if size >= 1024.0 {
            format!("{:.1} KB", size / 1024.0)
        } else {
            format!("{} B", metadata.len())
        }
    }

    fn file_label(path: &Path) -> String {
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled")
            .to_string()
    }

    fn generic_document(title: &str, body: &str) -> String {
        format!(
            r#"<!DOCTYPE html><html><head><meta charset="utf-8">
<meta name="color-scheme" content="light dark">
<style>
:root {{ color-scheme: light dark; }}
* {{ box-sizing: border-box; }}
body {{
  margin: 0;
  padding: 28px 32px;
  font-family: system-ui, -apple-system, "Segoe UI", sans-serif;
  color: #1f2937;
  background: #fff;
}}
main {{ max-width: 920px; margin: 0 auto; }}
h1 {{ margin: 0 0 6px; font-size: 24px; line-height: 1.2; }}
.meta {{ color: #6b7280; font-size: 13px; margin-bottom: 22px; }}
.grid {{ display: grid; grid-template-columns: max-content 1fr; gap: 8px 18px; }}
.label {{ color: #6b7280; font-size: 12px; text-transform: uppercase; letter-spacing: 0; }}
.value {{ min-width: 0; word-break: break-word; }}
.section {{ margin: 20px 0; padding-top: 16px; border-top: 1px solid #e5e7eb; }}
.card {{ border: 1px solid #e5e7eb; border-radius: 8px; padding: 16px; margin: 12px 0; }}
pre {{
  white-space: pre-wrap;
  word-break: break-word;
  background: #f6f8fa;
  border-radius: 6px;
  padding: 12px 14px;
  overflow: auto;
}}
table {{ border-collapse: collapse; width: 100%; margin: 12px 0; }}
th, td {{ border: 1px solid #e5e7eb; padding: 8px 10px; text-align: left; vertical-align: top; }}
th {{ background: #f9fafb; color: #374151; }}
@media (prefers-color-scheme: dark) {{
  body {{ background: #1a1a1a; color: #d4d4d4; }}
  .meta, .label {{ color: #9ca3af; }}
  .section, .card, th, td {{ border-color: #333; }}
  th {{ background: #252525; color: #d4d4d4; }}
  pre {{ background: #252525; }}
}}
</style></head><body><main>
<h1>{title}</h1>
{body}
</main></body></html>"#,
            title = html_escape::encode_text(title),
            body = body
        )
    }

    fn current_ext(&self) -> String {
        self.current_path
            .borrow()
            .as_ref()
            .map(|path| Self::ext_lower(path))
            .unwrap_or_default()
    }

    fn current_supports_fidelity(&self) -> bool {
        crate::fidelity::is_supported(&self.current_ext())
    }

    fn show_web(&self) {
        self.status_spinner.stop();
        self.stack.set_visible_child_name("web");
    }

    fn show_status(&self, message: &str, spinning: bool) {
        self.status_label.set_text(message);
        if spinning {
            self.status_spinner.start();
        } else {
            self.status_spinner.stop();
        }
        self.stack.set_visible_child_name("status");
    }

    fn show_fidelity_pdf(&self, path: &Path) {
        self.fidelity_pdf.load(path);
        self.stack.set_visible_child_name("pdf");
        self.status_spinner.stop();
    }

    fn load_iwork_file(&self, path: &Path) {
        match Self::find_iwork_preview(path) {
            Ok(IWorkPreview::Pdf(pdf_path, temp_dir)) => {
                self.show_fidelity_pdf(&pdf_path);
                if let Some(temp_dir) = temp_dir {
                    self.temp_dirs.borrow_mut().push(temp_dir);
                }
            }
            Ok(IWorkPreview::Image(image_path, temp_dir)) => {
                let uri = Self::file_uri(&image_path);
                let title = format!("{} preview", Self::file_label(path));
                let body = format!(
                    r#"<div class="meta">QuickLook thumbnail preview</div>
<div class="section"><img src="{}" alt="Preview" style="max-width:100%;height:auto;display:block;margin:0 auto;"></div>"#,
                    html_escape::encode_double_quoted_attribute(&uri)
                );
                self.webview
                    .load_html(&Self::generic_document(&title, &body), Some(&uri));
                if let Some(temp_dir) = temp_dir {
                    self.temp_dirs.borrow_mut().push(temp_dir);
                }
            }
            Err(message) => self.load_package_summary(path, "iWork preview unavailable", &message),
        }
    }

    fn find_iwork_preview(path: &Path) -> Result<IWorkPreview, String> {
        if path.is_dir() {
            for candidate in [
                "QuickLook/Preview.pdf",
                "preview.pdf",
                "Preview.pdf",
                "QuickLook/Thumbnail.jpg",
                "QuickLook/Thumbnail.png",
            ] {
                let candidate_path = path.join(candidate);
                if !candidate_path.is_file() {
                    continue;
                }
                if candidate.ends_with(".pdf") {
                    return Ok(IWorkPreview::Pdf(candidate_path, None));
                }
                return Ok(IWorkPreview::Image(candidate_path, None));
            }
            return Err("No QuickLook/Preview.pdf or thumbnail found in the package.".to_string());
        }

        let file = File::open(path).map_err(|e| format!("Failed to open package: {e}"))?;
        let mut archive =
            zip::ZipArchive::new(file).map_err(|e| format!("Failed to read package: {e}"))?;
        for candidate in [
            "QuickLook/Preview.pdf",
            "preview.pdf",
            "Preview.pdf",
            "QuickLook/Thumbnail.jpg",
            "QuickLook/Thumbnail.png",
        ] {
            let Ok(mut entry) = archive.by_name(candidate) else {
                continue;
            };
            let temp_dir = tempfile::Builder::new()
                .prefix("anyview-iwork-")
                .tempdir()
                .map_err(|e| format!("Failed to create temp directory: {e}"))?;
            let file_name = Path::new(candidate)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Preview");
            let target = temp_dir.path().join(file_name);
            let mut out =
                File::create(&target).map_err(|e| format!("Failed to extract preview: {e}"))?;
            std::io::copy(&mut entry, &mut out)
                .map_err(|e| format!("Failed to write preview: {e}"))?;
            if candidate.ends_with(".pdf") {
                return Ok(IWorkPreview::Pdf(target, Some(temp_dir)));
            }
            return Ok(IWorkPreview::Image(target, Some(temp_dir)));
        }
        Err("No QuickLook/Preview.pdf or thumbnail found in the package.".to_string())
    }

    fn load_package_summary(&self, path: &Path, title: &str, message: &str) {
        let mut rows = String::new();
        if path.is_dir() {
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten().take(80) {
                    let display_name = entry.file_name().to_string_lossy().into_owned();
                    let name = html_escape::encode_text(&display_name);
                    let kind = if entry.path().is_dir() {
                        "Folder"
                    } else {
                        "File"
                    };
                    rows.push_str(&format!(
                        "<tr><td>{kind}</td><td>{name}</td><td>-</td></tr>"
                    ));
                }
            }
        } else if let Ok(file) = File::open(path) {
            if let Ok(mut archive) = zip::ZipArchive::new(file) {
                for index in 0..archive.len().min(120) {
                    if let Ok(entry) = archive.by_index(index) {
                        let name = html_escape::encode_text(entry.name());
                        let kind = if entry.is_dir() { "Folder" } else { "File" };
                        rows.push_str(&format!(
                            "<tr><td>{kind}</td><td>{name}</td><td>{}</td></tr>",
                            entry.size()
                        ));
                    }
                }
            }
        }

        let table = if rows.is_empty() {
            "<p>No package entries could be listed.</p>".to_string()
        } else {
            "<table><thead><tr><th>Kind</th><th>Name</th><th>Size</th></tr></thead><tbody>"
                .to_string()
                + &rows
                + "</tbody></table>"
        };
        let body = format!(
            r#"<div class="meta">{} · {}</div>
<div class="card">{}</div>
<div class="section">{}</div>"#,
            html_escape::encode_text(&Self::file_label(path)),
            Self::file_size_label(path),
            html_escape::encode_text(message),
            table
        );
        self.webview
            .load_html(&Self::generic_document(title, &body), None);
    }

    fn load_font_file(&self, path: &Path) {
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) => {
                self.show_error(&format!("Failed to read font: {e}"));
                return;
            }
        };
        use base64::{engine::general_purpose::STANDARD, Engine as _};
        let b64 = STANDARD.encode(&bytes);
        let ext = Self::ext_lower(path);
        let mime = match ext.as_str() {
            "otf" => "font/otf",
            "ttc" => "font/collection",
            _ => "font/ttf",
        };
        let format_hint = match ext.as_str() {
            "otf" => "opentype",
            "ttc" => "truetype-collection",
            _ => "truetype",
        };
        let title = Self::file_label(path);
        let body = format!(
            r#"<style>
@font-face {{
  font-family: "AnyViewPreviewFont";
  src: url("data:{mime};base64,{b64}") format("{format_hint}");
}}
.sample {{
  font-family: "AnyViewPreviewFont", sans-serif;
  border-top: 1px solid #e5e7eb;
  padding: 18px 0;
}}
.big {{ font-size: 52px; line-height: 1.12; }}
.alphabet {{ font-size: 24px; line-height: 1.45; }}
.caption {{ color: #6b7280; font-size: 13px; margin: 12px 0 4px; }}
@media (prefers-color-scheme: dark) {{
  .sample {{ border-top-color: #333; }}
  .caption {{ color: #9ca3af; }}
}}
</style>
<div class="meta">{size}</div>
<div class="caption">Display</div>
<div class="sample big">The quick brown fox jumps over 1234567890</div>
<div class="caption">Alphabet</div>
<div class="sample alphabet">ABCDEFGHIJKLMNOPQRSTUVWXYZ<br>abcdefghijklmnopqrstuvwxyz<br>0123456789 !? &amp; @ # $ %</div>
<div class="caption">Multilingual</div>
<div class="sample alphabet">中文排版样张 · 日本語サンプル · 한국어 샘플<br>Résumé naïve façade · Καλημέρα · Пример текста</div>
<script>
if (document.fonts && document.fonts.load) {{
  document.fonts.load('16px AnyViewPreviewFont').catch(function() {{}});
}}
</script>"#,
            mime = mime,
            b64 = b64,
            format_hint = format_hint,
            size = Self::file_size_label(path)
        );
        self.webview
            .load_html(&Self::generic_document(&title, &body), None);
    }

    fn unfolded_lines(raw: &str) -> Vec<String> {
        let normalized = raw.replace("\r\n", "\n").replace('\r', "\n");
        let mut lines: Vec<String> = Vec::new();
        for line in normalized.split('\n') {
            if line.starts_with(' ') || line.starts_with('\t') {
                if let Some(previous) = lines.last_mut() {
                    previous.push_str(line.trim_start());
                }
            } else {
                lines.push(line.to_string());
            }
        }
        lines
    }

    fn split_content_line(line: &str) -> Option<(String, String, String)> {
        let (head, value) = line.split_once(':')?;
        let mut parts = head.split(';');
        let name = parts.next()?.to_ascii_uppercase();
        let params = parts.collect::<Vec<_>>().join(";");
        Some((name, params, value.to_string()))
    }

    fn decode_contact_text(value: &str) -> String {
        value
            .replace("\\n", "\n")
            .replace("\\N", "\n")
            .replace("\\,", ",")
            .replace("\\;", ";")
            .replace("\\\\", "\\")
    }

    fn contact_values(lines: &[String], key: &str) -> Vec<(String, String)> {
        lines
            .iter()
            .filter_map(|line| Self::split_content_line(line))
            .filter(|(name, _, _)| name == key)
            .map(|(_, params, value)| (params, Self::decode_contact_text(&value)))
            .collect()
    }

    fn first_contact_value(lines: &[String], key: &str) -> Option<String> {
        Self::contact_values(lines, key)
            .into_iter()
            .map(|(_, value)| value)
            .find(|value| !value.trim().is_empty())
    }

    fn contact_row(label: &str, value: &str) -> String {
        if value.trim().is_empty() {
            return String::new();
        }
        format!(
            r#"<div class="label">{}</div><div class="value">{}</div>"#,
            html_escape::encode_text(label),
            html_escape::encode_text(value).replace('\n', "<br>")
        )
    }

    fn load_vcard_file(&self, path: &Path) {
        let raw = match Self::read_text_lossy(path) {
            Ok(raw) => raw,
            Err(e) => {
                self.show_error(&e);
                return;
            }
        };
        let lines = Self::unfolded_lines(&raw);
        let mut cards: Vec<Vec<String>> = Vec::new();
        let mut current: Option<Vec<String>> = None;
        for line in lines {
            let upper = line.to_ascii_uppercase();
            if upper == "BEGIN:VCARD" {
                current = Some(Vec::new());
            } else if upper == "END:VCARD" {
                if let Some(card) = current.take() {
                    cards.push(card);
                }
            } else if let Some(card) = current.as_mut() {
                card.push(line);
            }
        }

        if cards.is_empty() {
            let body = format!(
                r#"<div class="meta">{} · {}</div><pre>{}</pre>"#,
                html_escape::encode_text(&Self::file_label(path)),
                Self::file_size_label(path),
                html_escape::encode_text(&raw)
            );
            self.webview
                .load_html(&Self::generic_document("vCard", &body), None);
            return;
        }

        let mut body = format!(
            r#"<div class="meta">{} contact{}</div>"#,
            cards.len(),
            if cards.len() == 1 { "" } else { "s" }
        );
        for card in cards {
            let full_name = Self::first_contact_value(&card, "FN")
                .or_else(|| Self::first_contact_value(&card, "N"))
                .unwrap_or_else(|| "Unnamed contact".to_string());
            let org = Self::first_contact_value(&card, "ORG").unwrap_or_default();
            let title = Self::first_contact_value(&card, "TITLE").unwrap_or_default();
            let emails = Self::contact_values(&card, "EMAIL")
                .into_iter()
                .map(|(_, value)| value)
                .collect::<Vec<_>>()
                .join("\n");
            let phones = Self::contact_values(&card, "TEL")
                .into_iter()
                .map(|(_, value)| value)
                .collect::<Vec<_>>()
                .join("\n");
            let addresses = Self::contact_values(&card, "ADR")
                .into_iter()
                .map(|(_, value)| {
                    value
                        .split(';')
                        .filter(|part| !part.trim().is_empty())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .collect::<Vec<_>>()
                .join("\n");
            let urls = Self::contact_values(&card, "URL")
                .into_iter()
                .map(|(_, value)| value)
                .collect::<Vec<_>>()
                .join("\n");
            let note = Self::first_contact_value(&card, "NOTE").unwrap_or_default();

            body.push_str(&format!(
                r#"<div class="card"><h2>{}</h2><div class="grid">{}{}{}{}{}{}</div></div>"#,
                html_escape::encode_text(&full_name),
                Self::contact_row("Organization", &org),
                Self::contact_row("Title", &title),
                Self::contact_row("Email", &emails),
                Self::contact_row("Phone", &phones),
                Self::contact_row("Address", &addresses),
                Self::contact_row("URL", &urls)
            ));
            if !note.trim().is_empty() {
                body.push_str(&format!(
                    r#"<div class="section"><pre>{}</pre></div>"#,
                    html_escape::encode_text(&note)
                ));
            }
        }
        self.webview
            .load_html(&Self::generic_document("Contacts", &body), None);
    }

    fn format_ics_datetime(value: &str, params: &str) -> String {
        let mut text = value.to_string();
        if value.len() == 8 && value.chars().all(|c| c.is_ascii_digit()) {
            text = format!("{}-{}-{}", &value[0..4], &value[4..6], &value[6..8]);
        } else if value.len() >= 15 && value.as_bytes().get(8) == Some(&b'T') {
            text = format!(
                "{}-{}-{} {}:{}:{}",
                &value[0..4],
                &value[4..6],
                &value[6..8],
                &value[9..11],
                &value[11..13],
                &value[13..15]
            );
            if value.ends_with('Z') {
                text.push_str(" UTC");
            }
        }
        if let Some(tzid) = params
            .split(';')
            .find_map(|p| p.strip_prefix("TZID=").or_else(|| p.strip_prefix("tzid=")))
        {
            text.push_str(" ");
            text.push_str(tzid);
        }
        text
    }

    fn load_calendar_file(&self, path: &Path) {
        let raw = match Self::read_text_lossy(path) {
            Ok(raw) => raw,
            Err(e) => {
                self.show_error(&e);
                return;
            }
        };
        let lines = Self::unfolded_lines(&raw);
        let mut events: Vec<Vec<String>> = Vec::new();
        let mut current: Option<Vec<String>> = None;
        for line in lines {
            let upper = line.to_ascii_uppercase();
            if upper == "BEGIN:VEVENT" {
                current = Some(Vec::new());
            } else if upper == "END:VEVENT" {
                if let Some(event) = current.take() {
                    events.push(event);
                }
            } else if let Some(event) = current.as_mut() {
                event.push(line);
            }
        }

        if events.is_empty() {
            let body = format!(
                r#"<div class="meta">{} · {}</div><pre>{}</pre>"#,
                html_escape::encode_text(&Self::file_label(path)),
                Self::file_size_label(path),
                html_escape::encode_text(&raw)
            );
            self.webview
                .load_html(&Self::generic_document("Calendar", &body), None);
            return;
        }

        let mut body = format!(
            r#"<div class="meta">{} event{}</div>"#,
            events.len(),
            if events.len() == 1 { "" } else { "s" }
        );
        for event in events {
            let value_for = |key: &str| -> Option<(String, String)> {
                event
                    .iter()
                    .filter_map(|line| Self::split_content_line(line))
                    .find(|(name, _, _)| name == key)
                    .map(|(_, params, value)| (params, Self::decode_contact_text(&value)))
            };
            let summary = value_for("SUMMARY")
                .map(|(_, value)| value)
                .unwrap_or_else(|| "Untitled event".to_string());
            let starts = value_for("DTSTART")
                .map(|(params, value)| Self::format_ics_datetime(&value, &params))
                .unwrap_or_default();
            let ends = value_for("DTEND")
                .map(|(params, value)| Self::format_ics_datetime(&value, &params))
                .unwrap_or_default();
            let location = value_for("LOCATION")
                .map(|(_, value)| value)
                .unwrap_or_default();
            let organizer = value_for("ORGANIZER")
                .map(|(_, value)| value)
                .unwrap_or_default();
            let url = value_for("URL").map(|(_, value)| value).unwrap_or_default();
            let description = value_for("DESCRIPTION")
                .map(|(_, value)| value)
                .unwrap_or_default();

            body.push_str(&format!(
                r#"<div class="card"><h2>{}</h2><div class="grid">{}{}{}{}{}</div>"#,
                html_escape::encode_text(&summary),
                Self::contact_row("Start", &starts),
                Self::contact_row("End", &ends),
                Self::contact_row("Location", &location),
                Self::contact_row("Organizer", &organizer),
                Self::contact_row("URL", &url)
            ));
            if !description.trim().is_empty() {
                body.push_str(&format!(
                    r#"<div class="section"><pre>{}</pre></div>"#,
                    html_escape::encode_text(&description)
                ));
            }
            body.push_str("</div>");
        }
        self.webview
            .load_html(&Self::generic_document("Calendar", &body), None);
    }

    fn load_model_file(&self, path: &Path) {
        let ext = Self::ext_lower(path);
        if ext == "obj" || ext == "stl" {
            self.load_mesh_preview(path, &ext);
            return;
        }

        if ext == "usdz" {
            self.load_package_summary(
                path,
                "USDZ model package",
                "Interactive USDZ rendering is not available on Linux yet. Package contents are listed below.",
            );
            return;
        }

        let raw = match Self::read_text_lossy(path) {
            Ok(raw) => raw,
            Err(e) => {
                self.show_error(&e);
                return;
            }
        };
        let max_chars = 200_000;
        let mut source = raw.chars().take(max_chars).collect::<String>();
        if raw.chars().count() > max_chars {
            source.push_str("\n\n[truncated]");
        }
        let body = format!(
            r#"<div class="meta">{} · {}</div>
<div class="card">Interactive rendering for .{} is not available on Linux yet. Showing the source/metadata view.</div>
<pre>{}</pre>"#,
            html_escape::encode_text(&Self::file_label(path)),
            Self::file_size_label(path),
            html_escape::encode_text(&ext),
            html_escape::encode_text(&source)
        );
        self.webview
            .load_html(&Self::generic_document("3D Model", &body), None);
    }

    fn load_mesh_preview(&self, path: &Path, ext: &str) {
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) => {
                self.show_error(&format!("Failed to read model: {e}"));
                return;
            }
        };
        use base64::{engine::general_purpose::STANDARD, Engine as _};
        let b64 = STANDARD.encode(&bytes);
        let title = html_escape::encode_text(&Self::file_label(path)).to_string();
        let size = Self::file_size_label(path);
        let html = r#"<!DOCTYPE html><html><head><meta charset="utf-8">
<meta name="color-scheme" content="light dark">
<style>
* { box-sizing: border-box; }
html, body { margin: 0; height: 100%; overflow: hidden; font-family: system-ui, -apple-system, "Segoe UI", sans-serif; }
body { background: #0f1115; color: #e5e7eb; display: flex; flex-direction: column; }
#bar { flex: 0 0 auto; display: flex; align-items: center; gap: 14px; padding: 8px 12px; background: #171a21; border-bottom: 1px solid rgba(255,255,255,0.08); font-size: 12px; color: #aeb6c2; }
#title { color: #f9fafb; font-weight: 600; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
#stats { margin-left: auto; white-space: nowrap; }
#wrap { position: relative; flex: 1; min-height: 0; }
canvas { display: block; width: 100%; height: 100%; background: radial-gradient(circle at 50% 42%, #1c2430, #0b0d12 72%); cursor: grab; }
canvas:active { cursor: grabbing; }
#error { position: absolute; inset: 0; display: none; align-items: center; justify-content: center; padding: 32px; text-align: center; color: #fca5a5; background: #111827; }
</style></head><body>
<div id="bar"><div id="title">__TITLE__</div><div>__SIZE__</div><div id="stats">Loading...</div></div>
<div id="wrap"><canvas id="c"></canvas><div id="error"></div></div>
<script>
const EXT = "__EXT__";
const B64 = "__B64__";
const canvas = document.getElementById('c');
const ctx = canvas.getContext('2d');
const stats = document.getElementById('stats');
const errorBox = document.getElementById('error');
let rx = -0.45, ry = 0.65, zoom = 1, dragging = false, lx = 0, ly = 0;

function bytesFromBase64(s) {
  const bin = atob(s);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}
function textFromBytes(bytes) {
  return new TextDecoder('utf-8', { fatal: false }).decode(bytes);
}
function parseOBJ(text) {
  const v = [], f = [];
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trim();
    if (!line || line[0] === '#') continue;
    const p = line.split(/\s+/);
    if (p[0] === 'v' && p.length >= 4) {
      v.push([parseFloat(p[1]), parseFloat(p[2]), parseFloat(p[3])]);
    } else if (p[0] === 'f' && p.length >= 4) {
      const idx = p.slice(1).map(part => {
        const n = parseInt(part.split('/')[0], 10);
        return n < 0 ? v.length + n : n - 1;
      }).filter(n => Number.isFinite(n) && n >= 0 && n < v.length);
      for (let i = 1; i + 1 < idx.length; i++) f.push([idx[0], idx[i], idx[i + 1]]);
    }
  }
  return { vertices: v, faces: f };
}
function parseSTL(bytes) {
  const binaryCount = bytes.length >= 84 ? new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength).getUint32(80, true) : 0;
  const looksBinary = bytes.length === 84 + binaryCount * 50;
  const v = [], f = [];
  if (looksBinary) {
    const dv = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
    let off = 84;
    for (let i = 0; i < binaryCount; i++) {
      off += 12;
      const base = v.length;
      for (let j = 0; j < 3; j++) {
        v.push([dv.getFloat32(off, true), dv.getFloat32(off + 4, true), dv.getFloat32(off + 8, true)]);
        off += 12;
      }
      f.push([base, base + 1, base + 2]);
      off += 2;
    }
    return { vertices: v, faces: f };
  }
  const text = textFromBytes(bytes);
  let tri = [];
  for (const line of text.split(/\r?\n/)) {
    const m = line.trim().match(/^vertex\s+([^\s]+)\s+([^\s]+)\s+([^\s]+)/i);
    if (!m) continue;
    tri.push([parseFloat(m[1]), parseFloat(m[2]), parseFloat(m[3])]);
    if (tri.length === 3) {
      const base = v.length;
      v.push(tri[0], tri[1], tri[2]);
      f.push([base, base + 1, base + 2]);
      tri = [];
    }
  }
  return { vertices: v, faces: f };
}
function normalize(mesh) {
  if (!mesh.vertices.length) throw new Error('No vertices found.');
  const min = [Infinity, Infinity, Infinity], max = [-Infinity, -Infinity, -Infinity];
  for (const p of mesh.vertices) for (let i = 0; i < 3; i++) { min[i] = Math.min(min[i], p[i]); max[i] = Math.max(max[i], p[i]); }
  const center = [(min[0]+max[0])/2, (min[1]+max[1])/2, (min[2]+max[2])/2];
  const span = Math.max(max[0]-min[0], max[1]-min[1], max[2]-min[2]) || 1;
  mesh.vertices = mesh.vertices.map(p => [(p[0]-center[0])/span, (p[1]-center[1])/span, (p[2]-center[2])/span]);
  return mesh;
}
function resize() {
  const dpr = window.devicePixelRatio || 1;
  const r = canvas.getBoundingClientRect();
  canvas.width = Math.max(1, Math.floor(r.width * dpr));
  canvas.height = Math.max(1, Math.floor(r.height * dpr));
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
}
function project(p, w, h) {
  const sx = Math.sin(rx), cx = Math.cos(rx), sy = Math.sin(ry), cy = Math.cos(ry);
  let x = p[0], y = p[1], z = p[2];
  let x1 = x * cy + z * sy, z1 = -x * sy + z * cy;
  let y1 = y * cx - z1 * sx;
  const scale = Math.min(w, h) * 0.78 * zoom;
  return [w / 2 + x1 * scale, h / 2 - y1 * scale];
}
let mesh;
function draw() {
  resize();
  const w = canvas.clientWidth, h = canvas.clientHeight;
  ctx.clearRect(0, 0, w, h);
  if (!mesh) return;
  ctx.lineWidth = 1;
  ctx.strokeStyle = 'rgba(226,232,240,.78)';
  const limit = Math.min(mesh.faces.length, 65000);
  ctx.beginPath();
  for (let i = 0; i < limit; i++) {
    const face = mesh.faces[i];
    const a = project(mesh.vertices[face[0]], w, h);
    const b = project(mesh.vertices[face[1]], w, h);
    const c = project(mesh.vertices[face[2]], w, h);
    ctx.moveTo(a[0], a[1]); ctx.lineTo(b[0], b[1]); ctx.lineTo(c[0], c[1]); ctx.lineTo(a[0], a[1]);
  }
  ctx.stroke();
}
try {
  const bytes = bytesFromBase64(B64);
  mesh = normalize(EXT === 'obj' ? parseOBJ(textFromBytes(bytes)) : parseSTL(bytes));
  stats.textContent = `${mesh.vertices.length.toLocaleString()} vertices · ${mesh.faces.length.toLocaleString()} triangles`;
  draw();
} catch (e) {
  errorBox.style.display = 'flex';
  errorBox.textContent = e && e.message ? e.message : String(e);
}
canvas.addEventListener('mousedown', e => { dragging = true; lx = e.clientX; ly = e.clientY; });
window.addEventListener('mouseup', () => dragging = false);
window.addEventListener('mousemove', e => {
  if (!dragging) return;
  ry += (e.clientX - lx) * 0.01;
  rx += (e.clientY - ly) * 0.01;
  lx = e.clientX; ly = e.clientY;
  draw();
});
canvas.addEventListener('wheel', e => {
  e.preventDefault();
  zoom = Math.max(0.25, Math.min(5, zoom * (e.deltaY < 0 ? 1.1 : 0.9)));
  draw();
}, { passive: false });
window.addEventListener('resize', draw);
</script></body></html>"#
            .replace("__TITLE__", &title)
            .replace("__SIZE__", &size)
            .replace("__EXT__", ext)
            .replace("__B64__", &b64);
        self.webview.load_html(&html, None);
    }

    fn begin_fidelity_conversion(&self, completion: Option<FidelityCompletion>) {
        let Some(path) = self.current_path.borrow().clone() else {
            if let Some(completion) = completion {
                completion(Err(FidelityError::UnsupportedExtension));
            }
            return;
        };

        if !crate::fidelity::is_supported(&Self::ext_lower(&path)) {
            if let Some(completion) = completion {
                completion(Err(FidelityError::UnsupportedExtension));
            }
            return;
        }

        if crate::libreoffice_cli::find_soffice().is_none() {
            *self.fidelity_enabled.borrow_mut() = false;
            self.show_status(&FidelityError::SofficeNotFound.to_string(), false);
            if let Some(completion) = completion {
                completion(Err(FidelityError::SofficeNotFound));
            }
            return;
        }

        if let Some(cached) = crate::fidelity::cached_pdf_path(&path) {
            self.show_fidelity_pdf(&cached);
            if let Some(completion) = completion {
                completion(Ok(()));
            }
            return;
        }

        let token = {
            let mut token = self.conversion_token.borrow_mut();
            *token += 1;
            *token
        };
        self.show_status("Generating fidelity preview...", true);

        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = sender.send(crate::fidelity::prepare_pdf(&path));
        });

        let receiver = Rc::new(RefCell::new(receiver));
        let stack = self.stack.clone();
        let spinner = self.status_spinner.clone();
        let label = self.status_label.clone();
        let pdf = self.fidelity_pdf.clone();
        let enabled = self.fidelity_enabled.clone();
        let token_ref = self.conversion_token.clone();
        let completion = Rc::new(RefCell::new(completion));

        glib::timeout_add_local(Duration::from_millis(100), move || {
            match receiver.borrow().try_recv() {
                Ok(result) => {
                    if *token_ref.borrow() == token {
                        spinner.stop();
                        match result {
                            Ok(pdf_path) => {
                                pdf.load(&pdf_path);
                                stack.set_visible_child_name("pdf");
                                if let Some(completion) = completion.borrow_mut().take() {
                                    completion(Ok(()));
                                }
                            }
                            Err(err) => {
                                *enabled.borrow_mut() = false;
                                label.set_text(&err.to_string());
                                stack.set_visible_child_name("status");
                                if let Some(completion) = completion.borrow_mut().take() {
                                    completion(Err(err));
                                }
                            }
                        }
                    } else if let Some(completion) = completion.borrow_mut().take() {
                        completion(Err(FidelityError::ConversionFailed(
                            "Fidelity conversion was superseded.".to_string(),
                        )));
                    }
                    glib::ControlFlow::Break
                }
                Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(mpsc::TryRecvError::Disconnected) => {
                    if *token_ref.borrow() == token {
                        spinner.stop();
                        let err = FidelityError::ConversionFailed(
                            "LibreOffice conversion worker stopped unexpectedly.".to_string(),
                        );
                        *enabled.borrow_mut() = false;
                        label.set_text(&err.to_string());
                        stack.set_visible_child_name("status");
                        if let Some(completion) = completion.borrow_mut().take() {
                            completion(Err(err));
                        }
                    } else if let Some(completion) = completion.borrow_mut().take() {
                        completion(Err(FidelityError::ConversionFailed(
                            "Fidelity conversion was superseded.".to_string(),
                        )));
                    }
                    glib::ControlFlow::Break
                }
            }
        });
    }

    fn lang_for(ext: &str) -> String {
        match ext {
            "rs" => "rust".into(),
            "py" => "python".into(),
            "js" | "jsx" => "javascript".into(),
            "ts" | "tsx" => "typescript".into(),
            "sh" | "bash" | "zsh" => "bash".into(),
            "yml" | "yaml" => "yaml".into(),
            "md" | "markdown" => "markdown".into(),
            "h" => "c".into(),
            "hpp" => "cpp".into(),
            "m" | "mm" => "objectivec".into(),
            "kt" => "kotlin".into(),
            "cs" => "csharp".into(),
            "rb" => "ruby".into(),
            "pl" => "perl".into(),
            "ex" | "exs" => "elixir".into(),
            "erl" => "erlang".into(),
            "hs" => "haskell".into(),
            "ml" => "ocaml".into(),
            "fs" => "fsharp".into(),
            "asm" | "s" => "x86asm".into(),
            "fish" => "shell".into(),
            "bat" | "cmd" => "dos".into(),
            "ps1" => "powershell".into(),
            "toml" | "cfg" | "conf" => "ini".into(),
            "plist" | "svg" | "dae" => "xml".into(),
            "sass" => "scss".into(),
            "rst" | "txt" | "log" => "plaintext".into(),
            "patch" => "diff".into(),
            "proto" => "protobuf".into(),
            "tex" | "sty" | "cls" => "latex".into(),
            "bib" | "bbl" => "tex".into(),
            "usd" => "plaintext".into(),
            other => other.to_string(),
        }
    }

    fn load_video_file(&self, path: &Path) {
        let video_uri = Self::file_uri(path);
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let filename_esc = html_escape::encode_text(filename).to_string();

        let html = format!(
            r#"<!DOCTYPE html><html><head><meta charset="UTF-8">
<style>
*{{margin:0;padding:0;box-sizing:border-box;}}
body{{background:#000;display:flex;flex-direction:column;height:100vh;overflow:hidden;}}
video{{flex:1;width:100%;min-height:0;background:#000;display:block;}}
#toolbar{{flex:0 0 auto;display:flex;align-items:center;gap:8px;padding:6px 12px;
         background:#111;font-family:system-ui,sans-serif;font-size:12px;color:#aaa;}}
#toolbar button{{padding:4px 10px;border:1px solid rgba(255,255,255,0.15);border-radius:5px;
               background:rgba(255,255,255,0.08);color:#ccc;font-size:12px;cursor:pointer;}}
#toolbar button:hover{{background:rgba(255,255,255,0.15);}}
#sub-label{{color:#666;font-style:italic;}}
#err{{color:#f87171;font-size:12px;display:none;padding:4px 8px;}}
</style></head><body>
<video id="v" controls preload="metadata">
  <source src="{video_uri}">
  <track id="sub-track" kind="subtitles" label="字幕" srclang="und" default>
</video>
<div id="toolbar">
  <button onclick="document.getElementById('f').click()">加载字幕…</button>
  <span id="sub-label">未加载字幕</span>
  <span id="err"></span>
  <input type="file" id="f" accept=".srt,.vtt,.ass,.ssa,.sub,.sbv" style="display:none">
</div>
<script>
function srtToVtt(s){{return 'WEBVTT\n\n'+s.replace(/\r\n/g,'\n').replace(/\r/g,'\n').replace(/(\d{{2}}:\d{{2}}:\d{{2}}),(\d{{3}})/g,'$1.$2').trim();}}
function assToVtt(s){{var vtt='WEBVTT\n\n',idx=1;s.split('\n').forEach(function(l){{var m=l.match(/^Dialogue:\s*\d+,([\d:.]+),([\d:.]+),[^,]*,[^,]*,[^,]*,[^,]*,[^,]*,[^,]*,(.*)/);if(!m)return;function t(ts){{var p=ts.split(':');return(p[0].length<2?'0'+p[0]:p[0])+':'+p[1]+':'+p[2].replace('.','.'); }}var tx=m[3].replace(/\{{[^}}]*\}}/g,'').replace(/<[^>]+>/g,'');vtt+=(idx++)+'\n'+t(m[1])+' --> '+t(m[2])+'\n'+tx+'\n\n';}});return vtt;}}
document.getElementById('f').onchange=function(e){{
  var file=e.target.files[0];if(!file)return;
  var err=document.getElementById('err');err.style.display='none';
  var reader=new FileReader();
  reader.onerror=function(){{err.textContent='读取失败';err.style.display='inline';}};
  reader.onload=function(ev){{
    var content=ev.target.result;
    var ext=file.name.split('.').pop().toLowerCase();
    var vtt;
    try{{if(ext==='vtt')vtt=content;else if(ext==='ass'||ext==='ssa')vtt=assToVtt(content);else vtt=srtToVtt(content);}}
    catch(ex){{err.textContent='解析失败: '+ex.message;err.style.display='inline';return;}}
    var blob=new Blob([vtt],{{type:'text/vtt'}});
    var url=URL.createObjectURL(blob);
    var track=document.getElementById('sub-track');
    var old=track.src;track.src=url;
    if(old&&old.startsWith('blob:'))URL.revokeObjectURL(old);
    var v=document.getElementById('v');
    for(var i=0;i<v.textTracks.length;i++)v.textTracks[i].mode='showing';
    document.getElementById('sub-label').textContent=file.name;
  }};
  reader.readAsText(file,'UTF-8');
}};
document.getElementById('v').onerror=function(){{
  document.getElementById('err').textContent='无法播放此格式（{filename}）— 取决于系统 GStreamer 插件';
  document.getElementById('err').style.display='inline';
}};
</script></body></html>"#,
            video_uri = video_uri,
            filename = filename_esc,
        );

        self.webview.load_html(&html, Some(&video_uri));
    }

    fn ffmpeg_path() -> Option<String> {
        let candidates = [
            "/usr/bin/ffmpeg",
            "/usr/local/bin/ffmpeg",
            "/opt/homebrew/bin/ffmpeg",
        ];
        for c in &candidates {
            if std::path::Path::new(c).exists() {
                return Some(c.to_string());
            }
        }
        // Check HOME/.local/bin/ffmpeg
        if let Ok(home) = std::env::var("HOME") {
            let p = format!("{}/.local/bin/ffmpeg", home);
            if std::path::Path::new(&p).exists() {
                return Some(p);
            }
        }
        None
    }

    fn transcode_and_play(&self, path: &Path) {
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let filename_esc = html_escape::encode_text(filename).to_string();

        let ffmpeg = match Self::ffmpeg_path() {
            Some(p) => p,
            None => {
                let html = format!(
                    r#"<!DOCTYPE html><html><head><meta charset="UTF-8"><style>
body{{background:#000;color:#aaa;font-family:system-ui,sans-serif;
     display:flex;flex-direction:column;justify-content:center;align-items:center;
     height:100vh;gap:12px;margin:0;}}
code{{color:#f87171;font-size:13px;}}
</style></head><body>
<div>无法播放 <b>{name}</b></div>
<div>需要 ffmpeg：<code>sudo apt install ffmpeg</code></div>
</body></html>"#,
                    name = filename_esc
                );
                self.webview.load_html(&html, None);
                return;
            }
        };

        // Show transcoding message (may not render before blocking, but consistent with tex pattern)
        let loading_html = format!(
            r#"<!DOCTYPE html><html><head><meta charset="UTF-8"><style>
body{{background:#000;color:#aaa;font-family:system-ui,sans-serif;
     display:flex;flex-direction:column;justify-content:center;align-items:center;
     height:100vh;gap:12px;margin:0;}}
</style></head><body>
<div>正在转码 <b>{name}</b>…</div>
</body></html>"#,
            name = filename_esc
        );
        self.webview.load_html(&loading_html, None);

        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("video");
        let out_path = std::env::temp_dir().join(format!("anyview-transcode-{}.mp4", stem));

        let result = std::process::Command::new(&ffmpeg)
            .args([
                "-y",
                "-i",
                path.to_str().unwrap_or(""),
                "-c:v",
                "copy",
                "-c:a",
                "aac",
                "-movflags",
                "faststart",
                out_path.to_str().unwrap_or(""),
            ])
            .output();

        match result {
            Ok(_) if out_path.exists() => {
                self.load_video_file(&out_path);
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                let html = format!(
                    r#"<!DOCTYPE html><html><head><meta charset="UTF-8"><style>
body{{background:#000;color:#aaa;font-family:system-ui,sans-serif;padding:24px;margin:0;}}
pre{{color:#f87171;font-size:11px;white-space:pre-wrap;margin-top:12px;}}
</style></head><body>
<div>转码失败：{name}</div>
<pre>{err}</pre>
</body></html>"#,
                    name = filename_esc,
                    err = html_escape::encode_text(&stderr)
                );
                self.webview.load_html(&html, None);
            }
            Err(e) => {
                self.show_error(&format!("ffmpeg 启动失败: {}", e));
            }
        }
    }

    fn load_subtitle_file(&self, path: &Path) {
        let raw = std::fs::read(path)
            .ok()
            .and_then(|b| {
                String::from_utf8(b.clone())
                    .ok()
                    .or_else(|| String::from_utf8_lossy(&b).to_string().into())
            })
            .unwrap_or_default();

        let source_escaped = html_escape::encode_text(&raw).to_string();
        let ext = Self::ext_lower(path);

        // Escape for JS template literal
        let js_source = raw
            .replace('\\', "\\\\")
            .replace('`', "\\`")
            .replace('$', "\\$");

        let html = format!(
            r#"<!DOCTYPE html><html><head><meta charset="UTF-8"><meta name="color-scheme" content="light dark">
<style>
*{{margin:0;padding:0;box-sizing:border-box;}}
body{{font-family:system-ui,-apple-system,"Segoe UI",sans-serif;font-size:14px;background:#fff;color:#1a1a1a;}}
#preview{{padding:0 0 40px;}}
.entry{{display:flex;gap:0;border-bottom:1px solid #f0f0f0;}}
.entry:hover{{background:#f9fafb;}}
.num{{flex:0 0 48px;padding:10px 8px 10px 16px;color:#aaa;font-size:12px;text-align:right;user-select:none;}}
.time{{flex:0 0 220px;padding:10px 12px;font-family:"JetBrains Mono","Fira Code",Menlo,monospace;font-size:11px;color:#888;white-space:nowrap;}}
.text{{flex:1;padding:10px 16px 10px 0;line-height:1.5;}}
.header{{position:sticky;top:0;z-index:10;padding:8px 16px;font-size:12px;color:#888;
         background:#fff;border-bottom:1px solid #e5e7eb;display:flex;gap:12px;}}
.header .th-num{{flex:0 0 48px;text-align:right;}}
.header .th-time{{flex:0 0 220px;padding-left:12px;}}
.header .th-text{{flex:1;}}
#source{{display:none;margin:0;padding:20px 24px;white-space:pre-wrap;word-wrap:break-word;
        font-family:"JetBrains Mono","Fira Code",Menlo,monospace;font-size:13px;line-height:1.5;}}
.toggle-btn{{position:fixed;top:12px;right:16px;z-index:9999;padding:4px 12px;
            border:1px solid rgba(0,0,0,0.15);border-radius:6px;
            background:rgba(255,255,255,0.9);color:#333;
            font-size:12px;font-family:system-ui,sans-serif;cursor:pointer;}}
@media(prefers-color-scheme:dark){{
body{{background:#1a1a1a;color:#d4d4d4;}}
.entry{{border-bottom-color:#2a2a2a;}}
.entry:hover{{background:#222;}}
.header{{background:#1a1a1a;border-bottom-color:#333;}}
.toggle-btn{{background:rgba(40,40,40,0.9);border-color:rgba(255,255,255,0.15);color:#ccc;}}
}}
</style></head><body>
<button class="toggle-btn" onclick="toggle()">&lt;/&gt; Source</button>
<div id="preview">
  <div class="header"><span class="th-num">#</span><span class="th-time">Timecode</span><span class="th-text">Text</span></div>
  <div id="entries"></div>
</div>
<pre id="source">{source}</pre>
<script>
var showing='preview';
function toggle(){{
var p=document.getElementById('preview');var s=document.getElementById('source');var btn=document.querySelector('.toggle-btn');
if(showing==='preview'){{p.style.display='none';s.style.display='block';btn.textContent='Preview';showing='source';}}
else{{s.style.display='none';p.style.display='block';btn.innerHTML='&lt;/&gt; Source';showing='preview';}}
}}
function esc(s){{return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');}}
function inlineStyle(s){{return s.replace(/<i>(.*?)<\/i>/g,'<em>$1</em>').replace(/<b>(.*?)<\/b>/g,'<b>$1</b>').replace(/<[^>]+>/g,'');}}
var raw=`{js_source}`;
var ext='{ext}';
var entries=[];
if(ext==='vtt'){{
  raw.replace(/\r\n/g,'\n').split(/\n\s*\n/).forEach(function(b){{
    b=b.trim();if(!b||b==='WEBVTT')return;
    var lines=b.split('\n');
    var ti=lines.findIndex(function(l){{return l.indexOf('-->')!==-1;}});
    if(ti===-1)return;
    var num=ti>0?lines[0]:String(entries.length+1);
    entries.push({{num:num,time:lines[ti],text:lines.slice(ti+1).join('<br>')}});
  }});
}}else{{
  raw.replace(/\r\n/g,'\n').split(/\n\s*\n/).forEach(function(b){{
    b=b.trim();if(!b)return;
    var lines=b.split('\n');if(lines.length<2)return;
    var time=lines[1].trim();if(time.indexOf('-->')===-1)return;
    entries.push({{num:lines[0].trim(),time:time,text:lines.slice(2).join('<br>')}});
  }});
}}
var c=document.getElementById('entries');
c.innerHTML=entries.length===0
  ?'<div style="padding:24px 16px;color:#888;font-size:13px;">No entries found — see source view.</div>'
  :entries.map(function(e){{return '<div class="entry"><div class="num">'+esc(e.num)+'</div><div class="time">'+esc(e.time)+'</div><div class="text">'+inlineStyle(e.text)+'</div></div>';}}).join('');
</script></body></html>"#,
            source = source_escaped,
            js_source = js_source,
            ext = ext,
        );

        self.webview.load_html(&html, None);
    }

    fn tectonic_path() -> Option<String> {
        let candidates = ["/usr/local/bin/tectonic", "/usr/bin/tectonic"];
        for c in &candidates {
            if std::fs::metadata(c).map(|m| m.is_file()).unwrap_or(false) {
                return Some(c.to_string());
            }
        }
        if let Ok(home) = std::env::var("HOME") {
            let local = format!("{}/.local/bin/tectonic", home);
            if std::fs::metadata(&local)
                .map(|m| m.is_file())
                .unwrap_or(false)
            {
                return Some(local);
            }
        }
        None
    }

    fn tex_source_html(escaped_source: &str, status_msg: &str, is_error: bool) -> String {
        let status_color = if is_error { "#b91c1c" } else { "#888" };
        let status_html = if status_msg.is_empty() {
            String::new()
        } else {
            format!(
                r#"<div style="position:fixed;top:12px;right:16px;z-index:9999;padding:6px 12px;font:12px system-ui,sans-serif;color:{};background:rgba(0,0,0,0.06);border-radius:4px;max-width:60%;white-space:pre-wrap;">{}</div>"#,
                status_color,
                html_escape::encode_text(status_msg)
            )
        };
        format!(
            r#"<!DOCTYPE html><html><head><meta charset="UTF-8"><meta name="color-scheme" content="light dark">
<style>
body{{margin:0;padding:20px 24px;font-family:"JetBrains Mono","Fira Code",Menlo,monospace;font-size:13px;background:#f8f9fa;color:#1a1a1a;}}
pre{{margin:0;line-height:1.5;white-space:pre-wrap;word-wrap:break-word;tab-size:4;}}
.hljs{{display:block;overflow-x:auto;padding:0;color:#333;background:transparent;}}
.hljs-comment,.hljs-quote{{color:#998;font-style:italic;}}
.hljs-keyword,.hljs-selector-tag{{color:#333;font-weight:bold;}}
.hljs-string,.hljs-doctag{{color:#d14;}}
.hljs-title,.hljs-section{{color:#900;font-weight:bold;}}
.hljs-built_in{{color:#0086b3;}}
@media(prefers-color-scheme:dark){{
body{{background:#1e1e1e;color:#d4d4d4;}}
.hljs{{color:#abb2bf;}}
.hljs-keyword{{color:#c678dd;}}
.hljs-string{{color:#98c379;}}
.hljs-built_in{{color:#e6c07b;}}
}}
</style>
<script>{hljs}</script>
<script>{hljs_latex}</script>
</head><body>
{status}
<pre><code class="language-latex">{source}</code></pre>
<script>if(window.hljs){{hljs.highlightAll();}}</script>
</body></html>"#,
            hljs = HIGHLIGHT_JS,
            hljs_latex = HLJS_LATEX_JS,
            status = status_html,
            source = escaped_source,
        )
    }

    fn tex_pdf_html(escaped_source: &str, pdf_uri: &str) -> String {
        format!(
            r#"<!DOCTYPE html><html><head><meta charset="UTF-8"><meta name="color-scheme" content="light dark">
<style>
*{{margin:0;padding:0;box-sizing:border-box;}}
body{{background:#fff;}}
iframe{{width:100%;height:100vh;border:none;display:block;}}
#source{{display:none;margin:0;padding:20px 24px;white-space:pre-wrap;word-wrap:break-word;
        font-family:"JetBrains Mono","Fira Code",Menlo,monospace;font-size:13px;line-height:1.5;
        tab-size:4;color:#1a1a1a;background:#f8f9fa;min-height:100vh;}}
.toggle-btn{{position:fixed;top:12px;right:16px;z-index:9999;padding:4px 12px;
            border:1px solid rgba(0,0,0,0.15);border-radius:6px;
            background:rgba(255,255,255,0.9);color:#333;
            font-size:12px;font-family:system-ui,sans-serif;cursor:pointer;
            backdrop-filter:blur(8px);}}
.toggle-btn:hover{{background:rgba(240,240,240,0.95);}}
.hljs{{display:block;overflow-x:auto;padding:0;color:#333;background:transparent;}}
.hljs-comment,.hljs-quote{{color:#998;font-style:italic;}}
.hljs-keyword,.hljs-selector-tag{{color:#333;font-weight:bold;}}
.hljs-string,.hljs-doctag{{color:#d14;}}
.hljs-title,.hljs-section{{color:#900;font-weight:bold;}}
.hljs-built_in{{color:#0086b3;}}
@media(prefers-color-scheme:dark){{
body{{background:#1a1a1a;}}
#source{{color:#d4d4d4;background:#1e1e1e;}}
.toggle-btn{{background:rgba(40,40,40,0.9);border-color:rgba(255,255,255,0.15);color:#ccc;}}
.hljs{{color:#abb2bf;}}
.hljs-keyword{{color:#c678dd;}}
.hljs-string{{color:#98c379;}}
.hljs-built_in{{color:#e6c07b;}}
}}
</style>
<script>{hljs}</script>
<script>{hljs_latex}</script>
</head><body>
<button class="toggle-btn" onclick="toggle()">&lt;/&gt; Source</button>
<iframe id="preview" src="{pdf}"></iframe>
<pre id="source"><code class="language-latex">{source}</code></pre>
<script>
var showing='preview';
function toggle(){{
var p=document.getElementById('preview');
var s=document.getElementById('source');
var btn=document.querySelector('.toggle-btn');
if(showing==='preview'){{p.style.display='none';s.style.display='block';btn.textContent='PDF';showing='source';if(window.hljs)hljs.highlightAll();}}
else{{s.style.display='none';p.style.display='block';btn.innerHTML='&lt;/&gt; Source';showing='preview';}}
}}
</script>
</body></html>"#,
            hljs = HIGHLIGHT_JS,
            hljs_latex = HLJS_LATEX_JS,
            pdf = pdf_uri,
            source = escaped_source,
        )
    }

    fn load_tex_file(&self, path: &Path) {
        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                self.show_error(&format!("Failed to read file: {}", e));
                return;
            }
        };
        let escaped = html_escape::encode_text(&source).to_string();

        // Subfile check — no \begin{document} means it's meant to be \input'd
        if !source.contains(r"\begin{document}") {
            let html = Self::tex_source_html(
                &escaped,
                "Subfile (no \\begin{document}) — source only",
                false,
            );
            self.webview.load_html(&html, None);
            return;
        }

        let tectonic = match Self::tectonic_path() {
            Some(p) => p,
            None => {
                let html = Self::tex_source_html(
                    &escaped,
                    "tectonic not found — install: cargo install tectonic  or  apt install tectonic",
                    true,
                );
                self.webview.load_html(&html, None);
                return;
            }
        };

        // Show source + "Compiling…" while tectonic runs
        self.webview.load_html(
            &Self::tex_source_html(&escaped, "Compiling\u{2026}", false),
            None,
        );

        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        let tmp_dir = std::env::temp_dir().join(format!("anyview-tex-{}", unique));
        let _ = std::fs::create_dir_all(&tmp_dir);

        let output = std::process::Command::new(&tectonic)
            .args([
                "-Z",
                "continue-on-errors",
                "--outdir",
                tmp_dir.to_str().unwrap_or(""),
            ])
            .arg(path)
            .output();

        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        let pdf_path = tmp_dir.join(format!("{}.pdf", stem));

        if pdf_path.exists() {
            let pdf_uri = format!("file://{}", pdf_path.to_string_lossy());
            let html = Self::tex_pdf_html(&escaped, &pdf_uri);
            self.webview.load_html(&html, None);
        } else {
            let err_msg = output
                .ok()
                .and_then(|o| String::from_utf8(o.stderr).ok())
                .unwrap_or_else(|| "Compilation failed".to_string());
            let html = Self::tex_source_html(&escaped, err_msg.trim(), true);
            self.webview.load_html(&html, None);
        }
    }

    fn load_html_file(&self, path: &Path) {
        let uri = Self::file_uri(path);
        let content = match Self::read_text_lossy(path) {
            Ok(content) => content,
            Err(e) => {
                self.show_error(&e);
                return;
            }
        };
        let source = html_escape::encode_text(&content).to_string();
        let uri_attr = html_escape::encode_double_quoted_attribute(&uri).to_string();
        let html = format!(
            r#"<!DOCTYPE html><html><head><meta charset="utf-8">
<meta name="color-scheme" content="light dark">
<style>
* {{ box-sizing: border-box; }}
html, body {{ margin: 0; height: 100%; overflow: hidden; background: #fff; }}
iframe {{ width: 100%; height: 100%; border: 0; display: block; background: #fff; }}
#source {{
  display: none;
  margin: 0;
  min-height: 100%;
  overflow: auto;
  padding: 20px 24px;
  white-space: pre-wrap;
  word-break: break-word;
  font: 13px/1.5 "JetBrains Mono", "Fira Code", Menlo, Consolas, monospace;
  color: #1a1a1a;
  background: #f8f9fa;
}}
.toggle-btn {{
  position: fixed;
  top: 12px;
  right: 16px;
  z-index: 9999;
  padding: 5px 10px;
  border: 1px solid rgba(0,0,0,.14);
  border-radius: 6px;
  background: rgba(255,255,255,.92);
  color: #333;
  font: 12px system-ui, sans-serif;
  cursor: pointer;
}}
@media (prefers-color-scheme: dark) {{
  html, body {{ background: #1a1a1a; }}
  #source {{ background: #1e1e1e; color: #d4d4d4; }}
  .toggle-btn {{ background: rgba(40,40,40,.92); color: #ddd; border-color: rgba(255,255,255,.16); }}
}}
</style></head><body>
<button class="toggle-btn" onclick="toggle()">Source</button>
<iframe id="preview" src="{uri}"></iframe>
<pre id="source">{source}</pre>
<script>
let showing = 'preview';
function toggle() {{
  const preview = document.getElementById('preview');
  const source = document.getElementById('source');
  const button = document.querySelector('.toggle-btn');
  if (showing === 'preview') {{
    preview.style.display = 'none';
    source.style.display = 'block';
    button.textContent = 'Preview';
    showing = 'source';
  }} else {{
    source.style.display = 'none';
    preview.style.display = 'block';
    button.textContent = 'Source';
    showing = 'preview';
  }}
}}
</script></body></html>"#,
            uri = uri_attr,
            source = source
        );
        self.webview.load_html(&html, Some(&uri));
    }

    fn load_docx_file(&self, path: &Path) {
        // Render .docx natively in the browser via docx-preview (docxjs):
        // read the raw bytes, base64-encode them, and let the embedded
        // script rehydrate the bytes into a Blob + renderAsync.
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                self.show_error(&format!("Failed to read file: {}", e));
                return;
            }
        };

        use base64::{engine::general_purpose::STANDARD, Engine as _};
        let b64 = STANDARD.encode(&bytes);

        let html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<meta name="color-scheme" content="light dark">
<style>
  :root {{ color-scheme: light dark; }}
  html, body {{ margin: 0; padding: 0; background: #e5e7eb; }}
  #container {{ padding: 20px 0; }}
  #container .docx-wrapper {{ background: #9CA3AF; padding: 30px; display: flex; flex-flow: column; align-items: center; }}
  #container .docx-wrapper > section.docx {{ background: #fff; box-shadow: 0 0 10px rgba(0,0,0,0.5); margin-bottom: 30px; }}
  #container .docx {{ color: #000; }}
  @media (prefers-color-scheme: dark) {{
    html, body {{ background: #1a1a1a; }}
    #container .docx-wrapper {{ background: #2a2a2a; }}
  }}
  .status {{ font-family: system-ui, sans-serif; padding: 40px; color: #333; text-align: center; }}
  .status.err {{ color: #c00; }}
  @font-face {{ font-family: '宋体'; font-weight: normal; src: local('STSong-Light'), local('STSong'); }}
  @font-face {{ font-family: '宋体'; font-weight: bold; src: local('STSong'); }}
  @font-face {{ font-family: 'SimSun'; font-weight: normal; src: local('STSong-Light'), local('STSong'); }}
  @font-face {{ font-family: 'SimSun'; font-weight: bold; src: local('STSong'); }}
  @font-face {{ font-family: '微软雅黑'; font-weight: normal; src: local('WenQuanYi Micro Hei'), local('Noto Sans CJK SC'), local('PingFang SC'); }}
  @font-face {{ font-family: '微软雅黑'; font-weight: bold; src: local('WenQuanYi Micro Hei'), local('Noto Sans CJK SC Bold'), local('PingFang SC Semibold'); }}
  @font-face {{ font-family: 'Microsoft YaHei'; font-weight: normal; src: local('WenQuanYi Micro Hei'), local('Noto Sans CJK SC'); }}
  @font-face {{ font-family: '黑体'; font-weight: normal; src: local('WenQuanYi Zen Hei'), local('Noto Sans CJK SC'), local('STHeiti'); }}
  @font-face {{ font-family: 'SimHei'; font-weight: normal; src: local('WenQuanYi Zen Hei'), local('Noto Sans CJK SC'); }}
  @font-face {{ font-family: '楷体'; src: local('AR PL UKai CN'), local('STKaiti'); }}
  @font-face {{ font-family: 'KaiTi'; src: local('AR PL UKai CN'), local('STKaiti'); }}
  @font-face {{ font-family: '仿宋'; src: local('STFangsong'); }}
  @font-face {{ font-family: 'FangSong'; src: local('STFangsong'); }}
  @font-face {{ font-family: '等线'; src: local('Noto Sans CJK SC'), local('WenQuanYi Micro Hei'); }}
</style>
<script>{jszip}</script>
<script>{docxjs}</script>
</head>
<body>
<div id="container"><div class="status">Rendering…</div></div>
<script>
(function() {{
  var b64 = "{b64}";
  try {{
    var bin = atob(b64);
    var len = bin.length;
    var bytes = new Uint8Array(len);
    for (var i = 0; i < len; i++) bytes[i] = bin.charCodeAt(i);
    var blob = new Blob([bytes], {{ type: "application/vnd.openxmlformats-officedocument.wordprocessingml.document" }});
    var container = document.getElementById("container");
    container.innerHTML = "";
    docx.renderAsync(blob, container, null, {{
      className: "docx",
      inWrapper: true,
      ignoreWidth: false,
      ignoreHeight: false,
      ignoreFonts: false,
      breakPages: true,
      ignoreLastRenderedPageBreak: true,
      experimental: false,
      trimXmlDeclaration: true,
      useBase64URL: true,
      renderChanges: false,
      renderHeaders: true,
      renderFooters: true,
      renderFootnotes: true,
      renderEndnotes: true,
      debug: false
    }}).catch(function(err) {{
      container.innerHTML = '<div class="status err">Render failed: ' + (err && err.message ? err.message : err) + '</div>';
    }});
  }} catch (err) {{
    document.getElementById("container").innerHTML = '<div class="status err">Decode failed: ' + err.message + '</div>';
  }}
}})();
</script>
</body>
</html>"#,
            jszip = JSZIP_JS,
            docxjs = DOCX_PREVIEW_JS,
            b64 = b64,
        );

        let base_uri = Self::file_uri(path);
        self.webview.load_html(&html, Some(&base_uri));
    }

    fn load_xlsx_file(&self, path: &Path) {
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                self.show_error(&format!("Failed to read file: {}", e));
                return;
            }
        };

        use base64::{engine::general_purpose::STANDARD, Engine as _};
        let b64 = STANDARD.encode(&bytes);

        let html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<meta name="color-scheme" content="light dark">
<style>
* {{ box-sizing: border-box; }}
html, body {{
  margin: 0;
  padding: 0;
  height: 100vh;
  overflow: hidden;
  font-family: system-ui, -apple-system, "Segoe UI", sans-serif;
  background: #f9fafb;
  color: #1f2937;
}}
body {{ display: flex; flex-direction: column; }}
#tabs {{
  flex: 0 0 auto;
  display: flex;
  gap: 2px;
  overflow-x: auto;
  padding: 6px 8px 0;
  background: rgba(0,0,0,0.04);
  border-bottom: 1px solid rgba(0,0,0,0.10);
}}
.tab {{
  border: 0;
  border-radius: 4px 4px 0 0;
  background: transparent;
  color: inherit;
  cursor: pointer;
  font: inherit;
  font-size: 12px;
  padding: 6px 14px;
  white-space: nowrap;
}}
.tab:hover {{ background: rgba(0,0,0,0.06); }}
.tab.active {{ background: #fff; box-shadow: 0 -1px 0 rgba(0,0,0,0.05); font-weight: 600; }}
#scroll {{ flex: 1; overflow: auto; padding: 12px; }}
#content {{ display: inline-block; min-width: 100%; }}
#empty {{ padding: 24px; color: #6b7280; font-size: 13px; }}
table {{ border-collapse: collapse; background: #fff; box-shadow: 0 1px 2px rgba(0,0,0,0.06); }}
td, th {{
  border: 1px solid #e5e7eb;
  font-size: 13px;
  max-width: 480px;
  min-width: 60px;
  padding: 4px 8px;
  vertical-align: top;
  white-space: pre-wrap;
  word-break: break-word;
}}
thead tr {{ background: #f3f4f6; }}
thead th {{ font-weight: 600; position: sticky; top: 0; z-index: 1; }}
#status {{
  position: fixed;
  right: 12px;
  top: 8px;
  padding: 4px 10px;
  background: rgba(17,24,39,0.84);
  color: #fff;
  border-radius: 4px;
  font-size: 11px;
  pointer-events: none;
}}
#status.gone {{ display: none; }}
#status.error {{ background: #b91c1c; }}
@media (prefers-color-scheme: dark) {{
  html, body {{ background: #111827; color: #e5e7eb; }}
  #tabs {{ background: rgba(255,255,255,0.04); border-bottom-color: rgba(255,255,255,0.10); }}
  .tab:hover {{ background: rgba(255,255,255,0.06); }}
  .tab.active {{ background: #1f2937; }}
  table {{ background: #1f2937; }}
  td, th {{ border-color: #374151; }}
  thead tr {{ background: #0f172a; }}
}}
</style>
<script>{xlsx}</script>
</head>
<body>
<div id="tabs"></div>
<div id="scroll"><div id="content"><div id="empty">Parsing...</div></div></div>
<div id="status">Parsing...</div>
<script>
(function() {{
  const tabsEl = document.getElementById('tabs');
  const contentEl = document.getElementById('content');
  const statusEl = document.getElementById('status');
  const b64 = "{b64}";
  let wb;
  try {{
    const bin = atob(b64);
    const buf = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) buf[i] = bin.charCodeAt(i);
    wb = XLSX.read(buf, {{ type: 'array', cellDates: true }});
  }} catch (e) {{
    contentEl.innerHTML = '';
    statusEl.className = 'error';
    statusEl.textContent = 'Parse failed: ' + (e && e.message ? e.message : e);
    return;
  }}
  const sheets = wb.SheetNames;
  if (!sheets.length) {{
    contentEl.innerHTML = '<div id="empty">Empty workbook</div>';
    statusEl.classList.add('gone');
    return;
  }}
  function renderSheet(idx) {{
    const ws = wb.Sheets[sheets[idx]];
    const html = XLSX.utils.sheet_to_html(ws, {{ editable: false, header: '', footer: '' }});
    contentEl.innerHTML = html;
    const table = contentEl.querySelector('table');
    if (table) {{
      const firstRow = table.querySelector('tr');
      if (firstRow) {{
        const thead = document.createElement('thead');
        thead.appendChild(firstRow);
        table.insertBefore(thead, table.firstChild);
        firstRow.querySelectorAll('td').forEach(function(td) {{
          const th = document.createElement('th');
          for (const attr of td.attributes) th.setAttribute(attr.name, attr.value);
          th.innerHTML = td.innerHTML;
          td.replaceWith(th);
        }});
      }}
    }}
    tabsEl.querySelectorAll('.tab').forEach(function(tab) {{
      tab.classList.toggle('active', tab.dataset.idx === String(idx));
    }});
    const ref = ws['!ref'] || '-';
    statusEl.className = '';
    statusEl.textContent = sheets[idx] + ' · ' + ref;
    clearTimeout(window.__hideStatus);
    window.__hideStatus = setTimeout(function() {{ statusEl.classList.add('gone'); }}, 1800);
  }}
  if (sheets.length > 1) {{
    sheets.forEach(function(name, idx) {{
      const button = document.createElement('button');
      button.className = 'tab' + (idx === 0 ? ' active' : '');
      button.textContent = name;
      button.dataset.idx = idx;
      button.addEventListener('click', function() {{ renderSheet(idx); }});
      tabsEl.appendChild(button);
    }});
  }} else {{
    tabsEl.style.display = 'none';
  }}
  renderSheet(0);
}})();
</script>
</body>
</html>"#,
            xlsx = XLSX_JS,
            b64 = b64
        );

        let base_uri = Self::file_uri(path);
        self.webview.load_html(&html, Some(&base_uri));
    }

    fn load_docmod_file(&self, path: &Path) {
        match crate::docmod_cli::render(path) {
            Ok(html) => {
                // docmod produces a full HTML doc. Load directly so its
                // embedded styles/scripts run as-is.
                let base_uri = Self::file_uri(path);
                self.webview.load_html(&html, Some(&base_uri));
            }
            Err(msg) => {
                self.show_error(&msg);
            }
        }
    }

    fn load_markdown_file(&self, path: &Path) {
        let raw = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                self.show_error(&format!("Failed to read file: {}", e));
                return;
            }
        };

        let mut opts = Options::empty();
        opts.insert(Options::ENABLE_TABLES);
        opts.insert(Options::ENABLE_FOOTNOTES);
        opts.insert(Options::ENABLE_STRIKETHROUGH);
        opts.insert(Options::ENABLE_TASKLISTS);
        opts.insert(Options::ENABLE_SMART_PUNCTUATION);
        opts.insert(Options::ENABLE_HEADING_ATTRIBUTES);

        let parser = Parser::new_ext(&raw, opts);
        let mut rendered = String::new();
        cmark_html::push_html(&mut rendered, parser);

        let source = html_escape::encode_text(&raw).to_string();
        let body = format!(
            r#"<style>
.toggle-btn {{
  position: fixed;
  top: 12px;
  right: 16px;
  z-index: 9999;
  padding: 5px 10px;
  border: 1px solid rgba(0,0,0,.14);
  border-radius: 6px;
  background: rgba(255,255,255,.92);
  color: #333;
  font: 12px system-ui, sans-serif;
  cursor: pointer;
}}
#markdown-source {{ display: none; }}
@media (prefers-color-scheme: dark) {{
  .toggle-btn {{ background: rgba(40,40,40,.92); color: #ddd; border-color: rgba(255,255,255,.16); }}
}}
</style>
<button class="toggle-btn" onclick="toggleMarkdownSource()">Source</button>
<div id="markdown-preview">{rendered}</div>
<pre id="markdown-source">{source}</pre>
<script>
let __markdownShowing = 'preview';
function toggleMarkdownSource() {{
  const preview = document.getElementById('markdown-preview');
  const source = document.getElementById('markdown-source');
  const button = document.querySelector('.toggle-btn');
  if (__markdownShowing === 'preview') {{
    preview.style.display = 'none';
    source.style.display = 'block';
    button.textContent = 'Preview';
    __markdownShowing = 'source';
  }} else {{
    source.style.display = 'none';
    preview.style.display = 'block';
    button.textContent = 'Source';
    __markdownShowing = 'preview';
  }}
}}
</script>"#,
            rendered = rendered,
            source = source
        );

        let wrapped = Self::wrap_document(&body);
        let base_uri = Self::file_uri(path);
        self.webview.load_html(&wrapped, Some(&base_uri));
    }

    fn load_code_file(&self, path: &Path) {
        let raw = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                self.show_error(&format!("Failed to read file: {}", e));
                return;
            }
        };
        let ext = Self::ext_lower(path);
        let lang = Self::lang_for(&ext);
        let escaped = html_escape::encode_text(&raw).to_string();
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        let filename_escaped = html_escape::encode_text(&filename).to_string();
        let line_count = raw.lines().count().max(if raw.is_empty() { 0 } else { 1 });

        let body = format!(
            "<div class=\"file-header\">{} lines &middot; {}</div>\n<pre><code class=\"language-{}\">{}</code></pre>",
            line_count, filename_escaped, lang, escaped
        );

        let wrapped = Self::wrap_document(&body);
        let base_uri = Self::file_uri(path);
        self.webview.load_html(&wrapped, Some(&base_uri));
    }

    fn show_error(&self, message: &str) {
        self.show_web();
        let escaped = html_escape::encode_text(message).to_string();
        let html = format!(
            r#"<!DOCTYPE html><html><head><meta charset="utf-8">
<style>
body {{ font-family: system-ui, -apple-system, "Segoe UI", sans-serif; padding: 40px; color: #333; background: #fff; }}
h2 {{ color: #c00; }}
pre {{ white-space: pre-wrap; word-wrap: break-word; background: #f5f5f5; padding: 12px; border-radius: 6px; }}
@media (prefers-color-scheme: dark) {{
  body {{ background: #1a1a1a; color: #d4d4d4; }}
  pre {{ background: #252525; }}
}}
</style></head><body>
<h2>Error</h2>
<pre>{}</pre>
</body></html>"#,
            escaped
        );
        self.webview.load_html(&html, None);
    }

    /// Wraps rendered HTML body in the full template (head + scripts + styles).
    fn wrap_document(body_html: &str) -> String {
        format!(
            r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<meta name="color-scheme" content="light dark">
<style>
:root {{
  color-scheme: light dark;
}}
* {{ box-sizing: border-box; }}
body {{
  font-family: system-ui, -apple-system, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
  font-size: 15px;
  line-height: 1.6;
  color: #1a1a1a;
  background: #ffffff;
  max-width: 900px;
  margin: 0 auto;
  padding: 24px 32px;
}}
h1, h2, h3, h4, h5, h6 {{ margin: 1em 0 0.5em; line-height: 1.25; }}
h1 {{ font-size: 1.8em; border-bottom: 1px solid #e5e7eb; padding-bottom: 0.3em; }}
h2 {{ font-size: 1.4em; border-bottom: 1px solid #e5e7eb; padding-bottom: 0.3em; }}
h3 {{ font-size: 1.2em; }}
p {{ margin: 0.75em 0; }}
a {{ color: #2563eb; }}
code {{
  font-family: "JetBrains Mono", "Fira Code", "SF Mono", Menlo, Consolas, monospace;
  font-size: 0.9em;
  background: #f3f4f6;
  padding: 2px 5px;
  border-radius: 3px;
}}
pre {{
  background: #f6f8fa;
  border-radius: 6px;
  overflow-x: auto;
  margin: 1em 0;
  padding: 12px 14px;
  line-height: 1.5;
}}
pre code {{
  background: none;
  padding: 0;
  font-size: 0.88em;
  border-radius: 0;
}}
blockquote {{
  margin: 1em 0;
  padding: 0 1em;
  border-left: 4px solid #d1d5db;
  color: #6b7280;
}}
table {{ border-collapse: collapse; width: 100%; margin: 1em 0; }}
th, td {{ border: 1px solid #d1d5db; padding: 8px 12px; text-align: left; }}
th {{ background: #f9fafb; font-weight: 600; }}
img {{ max-width: 100%; }}
hr {{ border: none; border-top: 1px solid #e5e7eb; margin: 2em 0; }}
ul, ol {{ padding-left: 2em; }}
li {{ margin: 0.25em 0; }}
.mermaid {{ display: flex; justify-content: center; margin: 1em 0; }}
.mermaid svg {{ max-width: 100%; height: auto; }}
.file-header {{
  color: #888;
  font-size: 12px;
  margin-bottom: 12px;
  padding-bottom: 8px;
  border-bottom: 1px solid #e5e7eb;
}}
@media (prefers-color-scheme: dark) {{
  body {{ background: #1a1a1a; color: #d4d4d4; }}
  h1, h2 {{ border-bottom-color: #333; }}
  code {{ background: #2d2d2d; }}
  pre {{ background: #1e1e1e; }}
  blockquote {{ border-left-color: #555; color: #999; }}
  th, td {{ border-color: #444; }}
  th {{ background: #252525; }}
  hr {{ border-top-color: #333; }}
  a {{ color: #60a5fa; }}
  .file-header {{ border-bottom-color: #333; color: #888; }}
}}
</style>
<script>{hljs}</script>
<script>{mermaid}</script>
</head>
<body>
{body}
<script>
(function() {{
  // Convert mermaid code blocks to <div class="mermaid"> BEFORE mermaid runs.
  var blocks = document.querySelectorAll('pre > code.language-mermaid');
  for (var i = 0; i < blocks.length; i++) {{
    var code = blocks[i];
    var pre = code.parentNode;
    var div = document.createElement('div');
    div.className = 'mermaid';
    div.textContent = code.textContent;
    pre.parentNode.replaceChild(div, pre);
  }}
  try {{ if (window.hljs) hljs.highlightAll(); }} catch (e) {{}}
  try {{
    if (window.mermaid) {{
      var isDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
      mermaid.initialize({{ startOnLoad: true, theme: isDark ? 'dark' : 'default' }});
    }}
  }} catch (e) {{}}
}})();
</script>
</body>
</html>
"#,
            hljs = HIGHLIGHT_JS,
            mermaid = MERMAID_JS,
            body = body_html
        )
    }
}

impl Renderer for WebRenderer {
    fn widget(&self) -> gtk::Widget {
        self.stack.clone().upcast()
    }

    fn load(&self, path: &Path) {
        *self.current_path.borrow_mut() = Some(path.to_path_buf());
        self.clear_temp_dirs();
        self.load_normal(path);

        if *self.fidelity_enabled.borrow() && self.current_supports_fidelity() {
            self.begin_fidelity_conversion(None);
        }
    }

    fn set_zoom(&self, level: f64) {
        self.fidelity_pdf.set_zoom(level);
        if !self
            .stack
            .visible_child_name()
            .map(|name| name.as_str() == "pdf")
            .unwrap_or(false)
        {
            self.webview.set_zoom_level(level);
        }
    }

    fn supports_find(&self) -> bool {
        true
    }

    fn perform_find(&self, query: &str, forward: bool, completion: FindCompletion) {
        if self
            .stack
            .visible_child_name()
            .map(|name| name.as_str() == "pdf")
            .unwrap_or(false)
        {
            self.fidelity_pdf.perform_find(query, forward, completion);
            return;
        }

        let Some(controller) = self.webview.find_controller() else {
            completion(false);
            return;
        };

        if query.trim().is_empty() {
            completion(false);
            return;
        }

        let mut options = webkit::FindOptions::CASE_INSENSITIVE | webkit::FindOptions::WRAP_AROUND;
        if !forward {
            options |= webkit::FindOptions::BACKWARDS;
        }

        *self.pending_find.borrow_mut() = Some(completion);
        *self.last_find_query.borrow_mut() = query.to_string();
        controller.search(query, options.bits(), 1);
    }

    fn supports_fidelity(&self) -> bool {
        self.current_supports_fidelity()
    }

    fn fidelity_enabled(&self) -> bool {
        *self.fidelity_enabled.borrow()
    }

    fn set_fidelity(&self, enabled: bool, completion: FidelityCompletion) {
        if !self.current_supports_fidelity() {
            completion(Err(FidelityError::UnsupportedExtension));
            return;
        }

        if enabled {
            *self.fidelity_enabled.borrow_mut() = true;
            self.begin_fidelity_conversion(Some(completion));
        } else {
            *self.fidelity_enabled.borrow_mut() = false;
            *self.conversion_token.borrow_mut() += 1;
            self.show_web();
            if let Some(path) = self.current_path.borrow().clone() {
                self.load_normal(&path);
            }
            completion(Ok(()));
        }
    }
}

impl WebRenderer {
    fn load_normal(&self, path: &Path) {
        self.show_web();
        let ext = Self::ext_lower(path);
        match ext.as_str() {
            "docx" => self.load_docx_file(path),
            "docmod" | "doct" => self.load_docmod_file(path),
            "key" | "numbers" | "pages" => self.load_iwork_file(path),
            "xlsx" | "xls" => self.load_xlsx_file(path),
            "html" | "htm" => self.load_html_file(path),
            "md" | "markdown" => self.load_markdown_file(path),
            "tex" => self.load_tex_file(path),
            "ttf" | "otf" | "ttc" => self.load_font_file(path),
            "vcf" => self.load_vcard_file(path),
            "ics" => self.load_calendar_file(path),
            "stl" | "obj" | "usdz" | "usd" | "dae" => self.load_model_file(path),
            "srt" | "vtt" | "ass" | "ssa" | "sub" | "sbv" => self.load_subtitle_file(path),
            "mp4" | "mov" | "m4v" | "webm" | "m2ts" | "ts" | "3gp" => self.load_video_file(path),
            "mkv" | "avi" | "flv" | "wmv" | "ogv" | "rmvb" | "rm" | "asf" | "vob" | "divx"
            | "f4v" => self.transcode_and_play(path),
            _ => self.load_code_file(path),
        }
    }
}
