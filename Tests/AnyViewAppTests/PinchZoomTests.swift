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
}
