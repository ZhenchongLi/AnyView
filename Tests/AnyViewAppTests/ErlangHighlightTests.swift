import XCTest
@testable import AnyViewApp

final class ErlangHighlightTests: XCTestCase {
    // Acceptance criterion #1 (issue #23): the highlight.js Erlang grammar must be
    // shipped as its own resource file in the macOS resource bundle, readable from
    // Bundle.module, with non-empty content. WebRenderer.hljsErlangScript is the
    // static property that reads `hljs-erlang.js` out of Bundle.module (mirroring
    // hljsLatexScript). Reading it non-empty proves both that the resource was
    // bundled and that its content is non-empty.
    func test_hljsErlangScript_isBundledAndNonEmpty() {
        XCTAssertFalse(
            WebRenderer.hljsErlangScript.isEmpty,
            "WebRenderer.hljsErlangScript must read a non-empty hljs-erlang.js out of Bundle.module; an empty value means the Erlang grammar resource was not bundled"
        )
    }
}
