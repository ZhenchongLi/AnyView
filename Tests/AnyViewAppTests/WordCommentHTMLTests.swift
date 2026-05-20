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

    func test_buildDocxHTML_containsCommentSidebarScaffold() {
        let html = buildDocxHTML(
            base64: "UEsDBAoAAAAAAA==",
            jszipScript: "/* stub jszip */",
            docxPreviewScript: "/* stub docx-preview */"
        )
        XCTAssertTrue(
            html.contains(".docx-comments-rail"),
            "Expected docx HTML to define the right-side comment rail CSS selector"
        )
        XCTAssertTrue(
            html.contains("moveCommentNodesToRail"),
            "Expected docx HTML to include the JavaScript hook that moves docx-preview comment nodes into the rail"
        )
    }

    // Regression pin (issue #9): the production `.docx` path must obtain its
    // HTML from `buildDocxHTML` so the comment sidebar scaffold actually reaches
    // the WKWebView. `loadDocxContent` writes straight to a WKWebView, so the
    // only observable seam from a unit test is the production source itself:
    // `loadDocxContent` must delegate to `buildDocxHTML` rather than building a
    // divergent inline HTML string that omits the scaffold.
    func test_loadDocxContent_delegatesToBuildDocxHTML() throws {
        let source = try String(
            contentsOf: webRendererSourceURL(),
            encoding: .utf8
        )
        let body = try XCTUnwrap(
            loadDocxContentBody(in: source),
            "Could not locate the loadDocxContent function body in WebRenderer.swift"
        )
        XCTAssertTrue(
            body.contains("buildDocxHTML("),
            "loadDocxContent must obtain its HTML from buildDocxHTML so the comment sidebar scaffold reaches the WKWebView, not from a divergent inline HTML string"
        )
        XCTAssertFalse(
            body.contains("@font-face { font-family: 'SimSun'"),
            "loadDocxContent must not keep its own inline copy of the docx HTML; that markup belongs to buildDocxHTML"
        )
    }

    // Acceptance criterion #5 (issue #9): the `.docmod`/`.doct` path must fetch
    // a comment-bearing document from a docmod command other than `render`,
    // which strips comments. The argument list comes from a function; this test
    // asserts that list is NOT `["render", <path>]` and reads with `"read"`.
    func test_docmodReadArguments_notRender() {
        let path = "/tmp/example.docmod"
        let args = DocmodCLI.docmodReadArguments(path: path)
        XCTAssertNotEqual(
            args,
            ["render", path],
            "docmodReadArguments must not invoke `render`, which strips comments from the document"
        )
        XCTAssertEqual(
            args.first,
            "read",
            "docmodReadArguments must fetch the comment-bearing document via the `read` subcommand"
        )
    }

    // Acceptance criterion #5 (issue #9), behavioral half: the `.docmod`/`.doct`
    // production paths must obtain their HTML from the comment-bearing `read`
    // invocation, NOT from `DocmodCLI.render` (i.e. `["render", <path>]`), which
    // strips comments. `loadDocmodContent`/`loadDoctContent` write straight to a
    // WKWebView, so the only observable seam from a unit test is the production
    // source itself: neither method may call `DocmodCLI.render`; both must route
    // through the read path so comment authors and text reach the web view.
    func test_loadDocmodAndDoctContent_useReadNotRender() throws {
        let source = try String(
            contentsOf: webRendererSourceURL(),
            encoding: .utf8
        )
        let docmodBody = try XCTUnwrap(
            functionBody(named: "loadDocmodContent", in: source),
            "Could not locate the loadDocmodContent function body in WebRenderer.swift"
        )
        let doctBody = try XCTUnwrap(
            functionBody(named: "loadDoctContent", in: source),
            "Could not locate the loadDoctContent function body in WebRenderer.swift"
        )
        XCTAssertFalse(
            docmodBody.contains("DocmodCLI.render"),
            "loadDocmodContent must not call DocmodCLI.render, which strips comments; it must fetch the comment-bearing document via the read path"
        )
        XCTAssertFalse(
            doctBody.contains("DocmodCLI.render"),
            "loadDoctContent must not call DocmodCLI.render, which strips comments; it must fetch the comment-bearing document via the read path"
        )
        XCTAssertTrue(
            docmodBody.contains("DocmodCLI.readHTML"),
            "loadDocmodContent must obtain its HTML from the read path (DocmodCLI.readHTML) so comments reach the web view"
        )
        XCTAssertTrue(
            doctBody.contains("DocmodCLI.readHTML"),
            "loadDoctContent must obtain its HTML from the read path (DocmodCLI.readHTML) so comments reach the web view"
        )
    }

    /// Locates `Sources/AnyViewApp/WebRenderer.swift` relative to this test file.
    private func webRendererSourceURL(file: StaticString = #filePath) -> URL {
        // .../Tests/AnyViewAppTests/WordCommentHTMLTests.swift -> repo root.
        let testFile = URL(fileURLWithPath: "\(file)")
        let repoRoot = testFile
            .deletingLastPathComponent() // AnyViewAppTests
            .deletingLastPathComponent() // Tests
            .deletingLastPathComponent() // repo root
        return repoRoot
            .appendingPathComponent("Sources")
            .appendingPathComponent("AnyViewApp")
            .appendingPathComponent("WebRenderer.swift")
    }

    /// Returns the brace-balanced body of `loadDocxContent`, or nil if absent.
    private func loadDocxContentBody(in source: String) -> String? {
        functionBody(named: "loadDocxContent", in: source)
    }

    /// Returns the brace-balanced body of the named function, or nil if absent.
    private func functionBody(named name: String, in source: String) -> String? {
        guard let signatureRange = source.range(of: "func \(name)(") else {
            return nil
        }
        guard let openBrace = source.range(of: "{", range: signatureRange.upperBound..<source.endIndex) else {
            return nil
        }
        var depth = 0
        var index = openBrace.lowerBound
        let bodyStart = openBrace.upperBound
        while index < source.endIndex {
            let ch = source[index]
            if ch == "{" { depth += 1 }
            else if ch == "}" {
                depth -= 1
                if depth == 0 {
                    return String(source[bodyStart..<index])
                }
            }
            index = source.index(after: index)
        }
        return nil
    }
}
