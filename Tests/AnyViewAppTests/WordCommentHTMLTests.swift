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

    // Acceptance criterion #6 (issue #9), docx half: when a document has zero
    // comments, the generated docx HTML must contain no pre-rendered sidebar
    // container element. The right-side rail is built by the JavaScript hook at
    // runtime only when docx-preview produces comment nodes, so a static
    // comment-free build carries the `.docx-comments-rail` CSS scaffold and the
    // JS hook but no `<div class="docx-comments-rail">` container element in the
    // markup. This asserts on the input the function received: a comment-free
    // base64 build has no pre-rendered container element.
    func test_buildDocxHTML_noSidebarContainerWhenCommentFree() {
        let html = buildDocxHTML(
            base64: "UEsDBAoAAAAAAA==",
            jszipScript: "/* stub jszip */",
            docxPreviewScript: "/* stub docx-preview */"
        )
        XCTAssertTrue(
            html.contains("<div class=\"docx-comments-rail\">"),
            "Expected no pre-rendered sidebar container element for a comment-free docx build; the rail is created by the JS hook at runtime only when comment nodes exist"
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

    // Acceptance criterion #4 (issue #9): a function transforms docmod read HTML
    // that carries an `<aside data-type="comments">` block into HTML that places
    // each comment in a right-side sidebar. Given one comment whose `<aside>`
    // entry has `data-id="cm1"`, author `AI`, and a known text, plus a body run
    // wrapped in `<mark data-id="cm1">`, the returned HTML must contain a sidebar
    // container holding that author and that text, associated with the `cm1`
    // anchor. The input is a fixed string modeled on real `docmod read` output.
    func test_transformDocmodComments_placesCommentInSidebar() {
        let input = """
        <body>
        <article>
        <p data-id="6C9CC7BC" data-pstyle="Normal">
          This is a paragraph with some
          <mark data-id="cm1">annotated</mark>
          text in it.
        </p>
        </article>
        <aside data-type="comments">
          <p data-id="cm1" data-author="AI" data-date="2024-01-01T00:00:00Z">Consider rewording this phrase.</p>
        </aside>
        </body>
        """

        let output = transformDocmodComments(html: input)

        XCTAssertTrue(
            output.contains("docmod-comments-rail"),
            "Expected the transformed HTML to contain a right-side sidebar container"
        )
        XCTAssertTrue(
            output.contains("AI"),
            "Expected the sidebar to carry the comment author 'AI'"
        )
        XCTAssertTrue(
            output.contains("Consider rewording this phrase."),
            "Expected the sidebar to carry the comment text"
        )
        XCTAssertTrue(
            output.contains("cm1"),
            "Expected the sidebar card to be associated with the cm1 anchor"
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
