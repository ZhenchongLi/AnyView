import Cocoa
import PDFKit

/// NSView that paginates a PDFDocument for `NSPrintOperation(view:printInfo:)`.
/// `rectForPage` returns each page's natural size at origin (0,0); `draw(_:)` consults
/// `NSPrintOperation.current.currentPage` to pick which page to render. The print system
/// drives the per-page clip + translate, so per-page positions in our own coord system
/// don't need to be unique.
final class PDFPrintView: NSView {
    private let document: PDFDocument
    private var pageSizes: [CGSize] = []

    init(document: PDFDocument) {
        self.document = document
        super.init(frame: .zero)
        var maxWidth: CGFloat = 0
        var maxHeight: CGFloat = 0
        for i in 0..<document.pageCount {
            guard let page = document.page(at: i) else {
                pageSizes.append(.zero); continue
            }
            let bounds = page.bounds(for: .mediaBox)
            let rotation = page.rotation
            let size: CGSize = (rotation == 90 || rotation == 270)
                ? CGSize(width: bounds.height, height: bounds.width)
                : bounds.size
            pageSizes.append(size)
            maxWidth = max(maxWidth, size.width)
            maxHeight = max(maxHeight, size.height)
        }
        frame = NSRect(x: 0, y: 0,
                       width: maxWidth > 0 ? maxWidth : 612,
                       height: maxHeight > 0 ? maxHeight : 792)
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }

    override var isFlipped: Bool { false }

    override func knowsPageRange(_ range: NSRangePointer) -> Bool {
        range.pointee = NSRange(location: 1, length: document.pageCount)
        return true
    }

    override func rectForPage(_ page: Int) -> NSRect {
        let idx = page - 1
        guard idx >= 0, idx < pageSizes.count else { return .zero }
        return NSRect(origin: .zero, size: pageSizes[idx])
    }

    override func draw(_ dirtyRect: NSRect) {
        guard let context = NSGraphicsContext.current?.cgContext,
              let op = NSPrintOperation.current else { return }
        let idx = op.currentPage - 1
        guard idx >= 0, idx < pageSizes.count,
              let page = document.page(at: idx) else { return }
        context.saveGState()
        // PDFPage.transform(for:) bakes in rotation + cropping so the page draws into
        // a (0,0)-origin rect of the size pageSizes[idx].
        context.concatenate(page.transform(for: .mediaBox))
        page.draw(with: .mediaBox, to: context)
        context.restoreGState()
    }
}

/// NSView that prints a single NSImage as a single page, fit to the printable area.
final class ImagePrintView: NSView {
    private let image: NSImage

    init(image: NSImage) {
        self.image = image
        let size = image.size == .zero ? CGSize(width: 612, height: 792) : image.size
        super.init(frame: NSRect(origin: .zero, size: size))
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }

    override var isFlipped: Bool { false }

    override func knowsPageRange(_ range: NSRangePointer) -> Bool {
        range.pointee = NSRange(location: 1, length: 1)
        return true
    }

    override func rectForPage(_ page: Int) -> NSRect { bounds }

    override func draw(_ dirtyRect: NSRect) {
        image.draw(in: bounds,
                   from: .zero,
                   operation: .sourceOver,
                   fraction: 1.0,
                   respectFlipped: true,
                   hints: nil)
    }
}

enum PrintHelpers {
    /// `scaleToFit`: shrink the entire view onto a single sheet (use for `ImagePrintView`).
    /// Otherwise: `.automatic` so the print system honors `knowsPageRange`/`rectForPage`.
    static func makePrintInfo(scaleToFit: Bool = false) -> NSPrintInfo {
        let info = NSPrintInfo.shared.copy() as! NSPrintInfo
        if scaleToFit {
            info.horizontalPagination = .fit
            info.verticalPagination = .fit
            info.isHorizontallyCentered = true
            info.isVerticallyCentered = true
        } else {
            info.horizontalPagination = .automatic
            info.verticalPagination = .automatic
        }
        return info
    }

    static func run(_ op: NSPrintOperation, attachedTo window: NSWindow?) {
        if let win = window {
            op.runModal(for: win, delegate: nil, didRun: nil, contextInfo: nil)
        } else {
            op.run()
        }
    }

    static func printPDFDocument(_ document: PDFDocument,
                                 jobTitle: String,
                                 attachedTo window: NSWindow?) {
        let view = PDFPrintView(document: document)
        let op = NSPrintOperation(view: view, printInfo: makePrintInfo())
        op.jobTitle = jobTitle
        run(op, attachedTo: window)
    }

    static func jobTitle(for filePath: String?) -> String {
        guard let filePath else { return "Document" }
        return URL(fileURLWithPath: filePath).lastPathComponent
    }
}
