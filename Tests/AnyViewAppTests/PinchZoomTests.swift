import XCTest
import PDFKit
@testable import AnyViewApp

final class PinchZoomTests: XCTestCase {

    /// Creates a real temp `.png` file so `RendererFactory` yields a real
    /// `ImageRenderer` and returns a `ViewerWindowController` pointed at it.
    private func makeController() throws -> ViewerWindowController {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("anyview-pinch-\(UUID().uuidString).png")
        try Data().write(to: url)
        addTeardownBlock { try? FileManager.default.removeItem(at: url) }
        return ViewerWindowController(filePath: url.path)
    }

    /// Writes a real one-page PDF and returns a controller pointed at it so
    /// `RendererFactory` yields a genuine `PDFRenderer` (no stub / no double).
    private func makePDFController() throws -> ViewerWindowController {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("anyview-pinch-\(UUID().uuidString).pdf")
        let data = NSMutableData()
        let consumer = CGDataConsumer(data: data as CFMutableData)!
        var box = CGRect(x: 0, y: 0, width: 200, height: 200)
        let ctx = CGContext(consumer: consumer, mediaBox: &box, nil)!
        ctx.beginPDFPage(nil)
        ctx.setFillColor(NSColor.red.cgColor)
        ctx.fill(CGRect(x: 10, y: 10, width: 100, height: 100))
        ctx.endPDFPage()
        ctx.closePDF()
        try (data as Data).write(to: url)
        addTeardownBlock { try? FileManager.default.removeItem(at: url) }
        return ViewerWindowController(filePath: url.path)
    }

    // Acceptance criterion #1 (issue #13): the preview area's pinch handler
    // lands on `ViewerWindowController.handleMagnification(_:)`, which must
    // route the gesture delta into the controller's zoom path. Feeding a
    // positive delta directly to that entry point must move `zoomLevel` off its
    // initial 1.0, proving the gesture entry point reaches the controller's
    // zoom logic.
    func test_magnification_triggersControllerZoom() throws {
        let controller = try makeController()

        controller.handleMagnification(0.5)

        XCTAssertNotEqual(
            controller.zoomLevel, 1.0,
            "Expected handleMagnification to move zoomLevel off its initial 1.0, proving the pinch entry point reaches the controller's zoom path"
        )
    }

    // Acceptance criterion #2 (issue #13): a pinch-open (positive delta) must
    // raise the SAME `zoomLevel` above its initial 1.0, and a pinch-close
    // (negative delta) must lower that same property. Reading `zoomLevel`
    // throughout proves there is no second copy of zoom state.
    func test_magnification_positiveIncreasesNegativeDecreasesSameZoomLevel() throws {
        let controller = try makeController()
        XCTAssertEqual(controller.zoomLevel, 1.0)

        controller.handleMagnification(0.5)
        let afterOpen = controller.zoomLevel
        XCTAssertGreaterThan(
            afterOpen, 1.0,
            "Pinch-open should increase zoomLevel above its initial 1.0"
        )

        controller.handleMagnification(-0.5)
        XCTAssertLessThan(
            controller.zoomLevel, afterOpen,
            "Pinch-close should decrease zoomLevel below the post-open value"
        )
    }

    // Acceptance criterion #3 (issue #13): the pinch-derived zoom factor is
    // clamped by the existing `minZoom` (0.5) and `maxZoom` (3.0); pinching all
    // the way must never cross those bounds. Feed a positive delta large enough
    // to exceed 3.0 and assert `zoomLevel` lands exactly on `maxZoom`; then feed
    // a negative delta large enough to drop below 0.5 and assert it lands
    // exactly on `minZoom`.
    func test_magnification_clampsToMinAndMaxZoom() throws {
        let controller = try makeController()

        controller.handleMagnification(10.0)
        XCTAssertEqual(
            controller.zoomLevel, ViewerWindowController.maxZoom,
            "A pinch-open large enough to exceed maxZoom must clamp at maxZoom"
        )

        controller.handleMagnification(-10.0)
        XCTAssertEqual(
            controller.zoomLevel, ViewerWindowController.minZoom,
            "A pinch-close large enough to drop below minZoom must clamp at minZoom"
        )
    }

    // Acceptance criterion #4 (issue #13): the pinch delta must reach the
    // current renderer's own real `setZoom(_:)` — no stub, no double. The
    // controller holds a genuine `PDFRenderer` from `RendererFactory`; after a
    // magnification the renderer's `PDFView.scaleFactor` (the externally
    // observable effect of `PDFRenderer.setZoom`) must equal the clamped
    // `zoomLevel`, proving the zoom actually landed on the real renderer.
    func test_magnification_appliesToRealRendererSetZoom() throws {
        let controller = try makePDFController()
        controller.showWindow(nil)

        // Drain the main queue so the async PDF document load completes before
        // we assert, so any layout/autoScales pass happens first.
        let drain = expectation(description: "drain main queue")
        DispatchQueue.main.async { drain.fulfill() }
        wait(for: [drain], timeout: 2.0)

        let renderer = try XCTUnwrap(
            controller.currentRenderer as? PDFRenderer,
            "Controller should hold a real PDFRenderer from RendererFactory"
        )
        let pdfView = try XCTUnwrap(renderer.view as? PDFView)

        controller.handleMagnification(10.0)

        XCTAssertEqual(
            controller.zoomLevel, ViewerWindowController.maxZoom,
            "Sanity: a large pinch-open should clamp zoomLevel at maxZoom"
        )
        XCTAssertEqual(
            pdfView.scaleFactor, controller.zoomLevel, accuracy: 0.0001,
            "The pinch zoom must reach the real PDFRenderer.setZoom, setting pdfView.scaleFactor to the clamped zoomLevel"
        )
    }

    // Regression pin (issue #13 review): the pinch receiving end must actually
    // exist and be wired. `showWindow` builds a `DropTargetView` as the window's
    // content view; that view must expose an `onMagnification` callback (same
    // pattern as `onDrop`), and `showWindow` must wire it to
    // `handleMagnification(_:)`. Without this wiring a trackpad pinch has no
    // receiver and `zoomLevel` never moves. The test reaches the real
    // `DropTargetView` off the window, asserts the callback is non-nil, then
    // invokes it with a positive delta and asserts `zoomLevel` rose — proving the
    // callback routes into the controller's zoom path.
    func test_showWindow_wiresDropTargetMagnificationToController() throws {
        let controller = try makePDFController()
        controller.showWindow(nil)

        let drain = expectation(description: "drain main queue")
        DispatchQueue.main.async { drain.fulfill() }
        wait(for: [drain], timeout: 2.0)

        let dropTarget = try XCTUnwrap(
            controller.window?.contentView as? DropTargetView,
            "showWindow should install a DropTargetView as the window content view"
        )
        let onMagnification = try XCTUnwrap(
            dropTarget.onMagnification,
            "showWindow must wire DropTargetView.onMagnification so trackpad pinch has a receiver"
        )

        XCTAssertEqual(controller.zoomLevel, 1.0)
        onMagnification(0.5)
        XCTAssertGreaterThan(
            controller.zoomLevel, 1.0,
            "Invoking the wired onMagnification callback must route into handleMagnification and raise zoomLevel"
        )
    }

    // Acceptance criterion #5 (issue #13): after a pinch changes the zoom, the
    // toolbar's percent label must read the same value `zoomLevel` computes to.
    // The label button is only created when `showWindow` installs the toolbar,
    // so the test installs the toolbar first, then feeds a magnification delta
    // and asserts the label button's `title` matches the current `zoomLevel`.
    func test_magnification_updatesToolbarPercentLabel() throws {
        let controller = try makePDFController()
        controller.showWindow(nil)

        // Drain the main queue so the toolbar finishes building its items
        // (the zoom label button) before we read its title.
        let drain = expectation(description: "drain main queue")
        DispatchQueue.main.async { drain.fulfill() }
        wait(for: [drain], timeout: 2.0)

        controller.handleMagnification(0.5)

        let expectedFromZoomLevel = "\(Int((controller.zoomLevel * 100).rounded()))%"
        XCTAssertEqual(
            controller.zoomLabelButtonTitle, expectedFromZoomLevel,
            "After a pinch, the toolbar percent label should equal the text the current zoomLevel computes to"
        )
    }
}
