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
        XCTAssertFalse(
            html.contains("<div class=\"docx-comments-rail\">"),
            "Expected no pre-rendered sidebar container element for a comment-free docx build; the rail is created by the JS hook at runtime only when comment nodes exist"
        )
    }

    // Acceptance criterion #7 (issue #9): the existing comment-free docx
    // rendering is preserved. The generated docx HTML must still contain the
    // `docx.renderAsync` call that drives docx-preview, and the CJK `@font-face`
    // block that maps Windows fonts (e.g. SimSun) to macOS equivalents. This is
    // a regression pin: both substrings are part of the production HTML today,
    // so the test exists to catch a future edit that drops either one.
    func test_buildDocxHTML_preservesRenderAsyncAndCJKFonts() {
        let html = buildDocxHTML(
            base64: "UEsDBAoAAAAAAA==",
            jszipScript: "/* stub jszip */",
            docxPreviewScript: "/* stub docx-preview */"
        )
        XCTAssertTrue(
            html.contains("docx.renderAsync"),
            "Expected docx HTML to preserve the docx.renderAsync call that drives docx-preview rendering"
        )
        XCTAssertTrue(
            html.contains("font-family: 'SimSun'"),
            "Expected docx HTML to preserve the CJK @font-face block mapping Windows fonts to macOS fonts"
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

    // Regression pin (issue #9, Review Real issue 2): the move of the docx HTML
    // into `buildDocxHTML` was sold as a pure extraction but dropped two
    // user-visible behaviors that trunk's inline string had. (1) On a render
    // failure the trunk surfaced the exception detail ('渲染失败: ' + message),
    // not a fixed message with no detail; a user who hits a failure must still
    // see what went wrong. (2) After the math fallback succeeded the banner was
    // cleared on a timer (`setTimeout(... 3000)`) instead of being pinned to the
    // screen for the life of the view. Both behaviors must survive the refactor.
    func test_buildDocxHTML_surfacesErrorDetailAndClearsMathFallbackBanner() {
        let html = buildDocxHTML(
            base64: "UEsDBAoAAAAAAA==",
            jszipScript: "/* stub jszip */",
            docxPreviewScript: "/* stub docx-preview */"
        )
        XCTAssertTrue(
            html.contains("渲染失败: "),
            "Expected the docx HTML to surface the exception detail on render failure ('渲染失败: ' + message), not a fixed message with no detail"
        )
        XCTAssertTrue(
            html.contains("e2.message ? e2.message : e2"),
            "Expected the render-failure handler to include the exception's message text so the user sees what went wrong"
        )
        XCTAssertTrue(
            html.contains("setTimeout(function() { status.textContent = ''; }, 3000)"),
            "Expected the math-fallback banner to clear after a 3-second timeout rather than staying pinned for the life of the view"
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

    // Issue #9 doct-chrome regression: on the `.doct` path the comment
    // `<aside>` block (with its injected `<style>`) is sliced out of
    // `<body>...</body>` and dropped into `buildDoctHtml`'s `.preview-frame`.
    // A global `body { margin-right: 280px }` rule inside that block then shifts
    // the entire doct page (header + metadata + preview), not just the preview.
    // The comment rail CSS must scope to its own elements and must NOT emit a
    // global `body` selector that mutates whatever page embeds the transformed
    // HTML.
    func test_transformDocmodComments_doesNotShiftEmbeddingBody() {
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

        XCTAssertFalse(
            output.contains("body { margin-right"),
            "The comment rail must not inject a global `body { margin-right }` rule; on the .doct path that rule is sliced into the doct viewer and shifts the entire doct page, not just the comment preview"
        )
    }

    // Acceptance criterion #6 (issue #9), docmod half: when docmod read HTML
    // carries no `<aside data-type="comments">` block, `transformDocmodComments`
    // returns the input unchanged with no sidebar container. This is the docmod
    // counterpart to test_buildDocxHTML_noSidebarContainerWhenCommentFree: a
    // comment-free input on the docmod path must not gain a
    // `docmod-comments-rail` container.
    func test_transformDocmodComments_noSidebarWhenNoAside() {
        let input = """
        <body>
        <article>
        <p data-id="6C9CC7BC" data-pstyle="Normal">
          This is a plain paragraph with no comments at all.
        </p>
        </article>
        </body>
        """

        let output = transformDocmodComments(html: input)

        XCTAssertFalse(
            output.contains("docmod-comments-rail"),
            "Expected no sidebar container for a comment-free docmod input; transformDocmodComments must return the input unchanged when there is no <aside data-type=\"comments\"> block"
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

    // Acceptance criterion #1 (issue #16): the HTML buildDocxHTML produces must
    // read the docx package's `word/comments.xml` in the NORMAL render path
    // (the one tied to the successful first `docx.renderAsync` call), not only
    // inside the math-formula `catch` fallback block that reads
    // `word/document.xml`. The current HTML reads `word/comments.xml` nowhere,
    // so the substring assertion is genuinely red first. To distinguish "normal
    // path reads comments.xml" from "math fallback reads document.xml" we anchor
    // on document order: the first successful render path ('docx.renderAsync'
    // then 'moveCommentNodesToRail') appears textually before the math-formula
    // fallback marker '数学公式', and the `word/comments.xml` read must fall in
    // that normal-path region — before the math fallback marker.
    func test_buildDocxHTML_readsCommentsXmlInNormalPath() {
        let html = buildDocxHTML(
            base64: "UEsDBAoAAAAAAA==",
            jszipScript: "/* stub jszip */",
            docxPreviewScript: "/* stub docx-preview */"
        )
        guard let commentsRange = html.range(of: "word/comments.xml") else {
            XCTFail("Expected docx HTML to read the docx package's word/comments.xml")
            return
        }
        guard let mathFallbackRange = html.range(of: "数学公式") else {
            XCTFail("Expected docx HTML to contain the math-formula fallback marker '数学公式' anchoring the catch block")
            return
        }
        XCTAssertLessThan(
            commentsRange.lowerBound, mathFallbackRange.lowerBound,
            "word/comments.xml must be read in the normal render path (before the math-formula fallback marker), not only inside the catch fallback that reads word/document.xml"
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
