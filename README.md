# AnyView

Native document viewer for macOS and Linux. View anything, locally, fast, faithful.

No Office, no Electron, no cloud. Just drop a file and see it.

| Light | Dark |
|-------|------|
| ![Code - Light](screenshots/code-swift.png) | ![Code - Dark](screenshots/code-dark.png) |
| ![Markdown](screenshots/markdown.png) | |

## Supported Formats

| Category | Formats | Engine |
|----------|---------|--------|
| PDF | pdf | PDFKit / Poppler |
| Word | docx, docmod, doct | WebKit docx-preview / docmod CLI |
| Presentations | pptx, ppt, key | Quick Look / LibreOffice PDF / iWork preview |
| Spreadsheets | xlsx, xls, numbers | SheetJS / iWork preview |
| Pages | pages | Quick Look / iWork preview |
| Images | png, jpg, gif, webp, tiff, bmp, ico, heic, svg | NSImageView / GTK Picture |
| Markdown | md, markdown | highlight.js + mermaid |
| HTML | html, htm | WKWebView / WebKitGTK |
| Code | 60+ languages | highlight.js |
| Data/Config | json, yaml, toml, xml, csv, plist, ini... | highlight.js |
| Media | mp3, m4a, wav, flac, aac, aiff, mp4, mov, m4v, avi, webm... | Quick Look / GStreamer / WebKit |
| 3D / Fonts / Calendar | stl, obj, usdz, usd, dae, ttf, otf, ttc, vcf, ics | Quick Look / Linux Web previews |

## Features

- Drag & drop to open
- Multi-tab browsing
- Zoom 50%--300%
- Light / Dark theme
- Auto-reload on file change
- Mermaid diagrams in Markdown
- HTML preview / source toggle
- Syntax highlighting for code

## Build

```bash
# requires macOS 13+, Swift 5.9+
swift build

# build .app bundle
./build-app.sh
```

### Linux

```bash
cd linux
cargo build
cargo build --release
```

Linux runtime/build dependencies are GTK 4, libadwaita, WebKitGTK 6, Poppler, and GStreamer plugins for media playback. `docmod` is optional for `docmod`/`doct` HTML preview. LibreOffice is optional for high-fidelity PDF preview of Office files and required for `pptx`/`ppt` preview. `ffmpeg` and `tectonic` are optional for transcoded video and LaTeX PDF preview.

## Architecture

Pluggable renderer protocol -- adding a new format is one file:

```
ViewerRenderer (protocol)
  |-- PDFRenderer        (PDFKit / Poppler)
  |-- ImageRenderer      (NSImageView / GTK Picture)
  |-- QuickLookRenderer  (QLPreviewView, macOS)
  |-- WebRenderer        (WKWebView / WebKitGTK -- docx/xlsx/md/html/code)
```

`ViewerWindowController` handles window, toolbar, file watching, zoom. Renderers handle rendering. ~213 lines of controller, zero `if isFormatX` branches.

## Requirements

- macOS 13+ for the Swift/AppKit app
- Linux with GTK 4, libadwaita, WebKitGTK 6, Poppler, and GStreamer for the Rust/GTK app
- [docmod](https://github.com/cove-apps/docmod) CLI (for docx/docmod/doct rendering, optional)
- LibreOffice (`soffice`/`libreoffice`) for Linux fidelity mode and presentation preview, optional except for pptx/ppt
