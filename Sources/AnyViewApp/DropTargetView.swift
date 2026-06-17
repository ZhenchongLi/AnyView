import Cocoa

/// A container view that accepts file drops and forwards them via a callback.
class DropTargetView: NSView {
    var onDrop: (([String]) -> Void)?

    /// Forwards a trackpad pinch's magnification increment (positive = pinch
    /// open, negative = pinch close) to whoever wires it up — same pattern as
    /// `onDrop`. `ViewerWindowController` sets this in `showWindow`.
    var onMagnification: ((CGFloat) -> Void)?

    override func magnify(with event: NSEvent) {
        onMagnification?(event.magnification)
    }

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        registerForDraggedTypes([.fileURL])
    }

    required init?(coder: NSCoder) {
        super.init(coder: coder)
        registerForDraggedTypes([.fileURL])
    }

    override func draggingEntered(_ sender: NSDraggingInfo) -> NSDragOperation {
        guard hasValidFiles(sender) else { return [] }
        return .copy
    }

    override func performDragOperation(_ sender: NSDraggingInfo) -> Bool {
        guard let items = sender.draggingPasteboard.readObjects(forClasses: [NSURL.self],
                options: [.urlReadingFileURLsOnly: true]) as? [URL] else {
            return false
        }
        let paths = items
            .filter { RendererFactory.allSupportedExtensions.contains($0.pathExtension.lowercased()) }
            .map { $0.path }
        guard !paths.isEmpty else { return false }
        onDrop?(paths)
        return true
    }

    private func hasValidFiles(_ sender: NSDraggingInfo) -> Bool {
        guard let items = sender.draggingPasteboard.readObjects(forClasses: [NSURL.self],
                options: [.urlReadingFileURLsOnly: true]) as? [URL] else {
            return false
        }
        return items.contains { RendererFactory.allSupportedExtensions.contains($0.pathExtension.lowercased()) }
    }
}
