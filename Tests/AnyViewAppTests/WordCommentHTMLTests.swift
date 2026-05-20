import XCTest
@testable import AnyViewApp

final class WordCommentHTMLTests: XCTestCase {
    func test_testTargetRuns() {
        XCTAssertEqual(1, 1)
    }

    func test_buildDocxHTML_isCallableStandalone() {
        let html = buildDocxHTML(
            base64: "UEsDBAoAAAAAAA==",
            jszipScript: "/* stub jszip */",
            docxPreviewScript: "/* stub docx-preview */"
        )
        XCTAssertFalse(html.isEmpty)
    }
}
