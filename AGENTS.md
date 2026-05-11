# AGENTS.md

This file provides guidance to coding agents when working with code in this repository.

## Project

AnyView is a native document viewer for macOS and Linux that opens arbitrary file types locally: no Office dependency for common preview paths, no Electron, no cloud.

- macOS: Swift + AppKit, single-target SPM executable, minimum macOS 13, Swift 5.9+.
- Linux: Rust + GTK 4/libadwaita/WebKitGTK/Poppler/GStreamer under `linux/`.

## Commands

```bash
# macOS
swift build                    # debug build
swift build -c release         # release build
./build-app.sh                 # build .build/AnyView.app (debug by default, or: ./build-app.sh release)
open .build/AnyView.app        # launch

# Linux
cd linux
cargo build
cargo check
cargo build --release
./install.sh                   # install user-local binary, desktop file, icon, MIME package
anyview <file>                 # launch installed app
```

There is no Swift test target, so `swift test` will fail. Linux currently verifies with `cargo check` plus manual launch/open checks. Manual verification should open real sample files of each supported extension and confirm they render.

`build-app.sh` bundles the `docmod` CLI into `Contents/MacOS/` if found (checks `$DOCMOD_PATH`, `~/.local/bin/docmod`, `~/.docmod/bin/docmod`, `PATH`). The app still builds without it; `.docx`/`.docmod`/`.doct` rendering just won't work.

`linux/install.sh` installs to `~/.local` by default and rewrites the installed desktop entry to use an absolute `Exec=/path/to/anyview %U` plus `TryExec`. Keep that behavior: graphical file managers such as Thunar may not have `~/.local/bin` in `PATH`, and relative `Exec=anyview %F` can make AnyView disappear from "Open With" lists.

## Architecture

The macOS core abstraction is the **`ViewerRenderer` protocol** (`Sources/AnyViewApp/ViewerRenderer.swift`): each renderer owns its `NSView`, declares a static `supportedExtensions` set, and implements `load(filePath:)` + `setZoom(_:)`. Four concrete renderers:

| Renderer | Backend | Handles |
|---|---|---|
| `PDFRenderer` | PDFKit | pdf |
| `ImageRenderer` | NSImageView | png/jpg/gif/webp/tiff/bmp/ico/heic/svg |
| `QuickLookRenderer` | QLPreviewView | Office/iWork/audio/video/3D/fonts/vcf/ics — anything macOS QuickLook handles |
| `WebRenderer` | WKWebView | docx/docmod/doct (via `docmod` CLI), html, markdown (highlight.js + mermaid), 60+ code languages, data/config formats |

`RendererFactory.renderer(for:)` dispatches by extension, falling back to `WebRenderer`. `RendererFactory.allSupportedExtensions` is the union used for the Open panel's allowed types and for rejecting unsupported extensions in `AppDelegate.openDocument(at:)`.

The Linux app mirrors that shape under `linux/src/` with a `Renderer` trait and `RendererFactory`. Current Linux renderers include PDF, image, media, Office, and Web renderers. The Web renderer handles Markdown/code/data, docx/docmod/doct, spreadsheets, iWork preview packages, fonts, vCard/calendar, and lightweight 3D previews. LibreOffice, ffmpeg, tectonic, and docmod are optional helper tools for higher-fidelity paths.

**Adding a format is usually one renderer plus metadata work**: add the extension to the appropriate renderer's supported extension list. If the format needs filesystem/app-launch awareness, update macOS `Sources/AnyViewApp/Info.plist` and Linux `linux/data/anyview.desktop` / `linux/data/anyview.xml`. `QuickLookRenderer` is the preferred macOS landing spot when macOS already previews the format natively.

`ViewerWindowController` owns the window, toolbar (zoom + appearance toggle), drop target, and file-watching. File changes are observed via `DispatchSource.makeFileSystemObjectSource` on the file descriptor, debounced 250 ms, then the renderer's `load(filePath:)` is called again on a background queue. `DropTargetView` routes new paths back up through `AppDelegate.openDocument(at:)` so dropped files open as new tabs (tabs are enabled via `NSWindow.tabbingIdentifier = "AnyView"`).

`WebRenderer` is the only renderer with meaningful complexity:
- `.docx`/`.docmod`/`.doct` are rendered by shelling out to the `docmod` CLI (`DocmodCLI.swift`). `DocmodCLI.findDocmod()` searches the app bundle's MacOS dir first, then `~/.local/bin`, `~/.docmod/bin`, `$DOCMOD_PATH`, `which`, and common install paths.
- Markdown/code use `highlight.min.js` and `mermaid.min.js` loaded from `Bundle.module` (SPM resource bundle under `Sources/AnyViewApp/Resources/`).
- Any zip-based formats use `ZipExtractor` (shells out to `/usr/bin/unzip` into a per-instance temp dir that is cleaned up on `cleanup()` / `deinit`). If you add a new zip-based format, wire its temp dir through the existing `tempDir` lock so reload cleanup still works.

`AppDelegate` is the single entry point for opening files: file-picker, drag-and-drop, and double-click all funnel through `openDocument(at:)`, which deduplicates already-open paths and activates the existing window instead of re-opening.

## Roadmap context

`docs/coverage-expansion.md` is the active design doc for broadening format coverage. It's organized in four phases (Phase 1: QuickLook piggybacking — largely done; Phase 2: WebRenderer text/structured formats like ipynb/epub/geojson; Phase 3: custom renderers; Phase 4: explicitly not planned). Consult it before adding a format — the phasing reflects a deliberate "don't reinvent wheels" policy.
