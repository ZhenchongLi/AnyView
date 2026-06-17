import XCTest
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
}
