import Cocoa
import PDFKit

/// Renders PDF files using macOS native PDFView.
class PDFRenderer: ViewerRenderer, SupportsFind, SupportsPrint {
    static let supportedExtensions: Set<String> = ["pdf"]

    private let pdfView: PDFView
    private var lastFindSelection: PDFSelection?

    var view: NSView { pdfView }

    init() {
        pdfView = PDFView(frame: .zero)
        pdfView.autoresizingMask = [.width, .height]
        pdfView.autoScales = true
        pdfView.displayMode = .singlePageContinuous
    }

    func load(filePath: String) {
        guard let document = PDFDocument(url: URL(fileURLWithPath: filePath)) else {
            DispatchQueue.main.async {
                let alert = NSAlert()
                alert.messageText = "Error"
                alert.informativeText = "Failed to load PDF: \(filePath)"
                alert.alertStyle = .critical
                alert.runModal()
            }
            return
        }
        DispatchQueue.main.async { [weak self] in
            self?.pdfView.document = document
            self?.lastFindSelection = nil
        }
    }

    func setZoom(_ level: CGFloat) {
        pdfView.scaleFactor = level
    }

    var canPrint: Bool { pdfView.document != nil }

    func runPrint(attachedTo window: NSWindow?) {
        guard let doc = pdfView.document else { return }
        PrintHelpers.printPDFDocument(doc,
                                      jobTitle: PrintHelpers.jobTitle(for: doc.documentURL?.path),
                                      attachedTo: window)
    }

    func performFind(query: String, forward: Bool, completion: @escaping (Bool) -> Void) {
        DispatchQueue.main.async { [weak self] in
            guard let self, let doc = self.pdfView.document else {
                completion(false); return
            }
            var options: NSString.CompareOptions = [.caseInsensitive]
            if !forward { options.insert(.backwards) }
            let match = doc.findString(query, fromSelection: self.lastFindSelection, withOptions: options)
                ?? doc.findString(query, fromSelection: nil, withOptions: options)
            if let match {
                self.lastFindSelection = match
                self.pdfView.setCurrentSelection(match, animate: true)
                self.pdfView.scrollSelectionToVisible(nil)
                completion(true)
            } else {
                completion(false)
            }
        }
    }
}
