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

    // Acceptance criterion #2 (issue #16): the JS buildDocxHTML produces must,
    // for each `w:comment` in `word/comments.xml`, append a card into the
    // `.docx-comments-rail` carrying that comment's body text. Criterion #1
    // already made the normal path read `word/comments.xml`, but the read JS
    // does nothing with the file's contents yet — it never iterates `w:comment`
    // nodes and never builds a card. This anchors on the comment-parsing region
    // (between the `word/comments.xml` read and the math-formula fallback
    // marker '数学公式'): in that region the JS must iterate over `w:comment`
    // (substring `w:comment`), pull each comment's body text (substring `w:t`),
    // and append a card into the rail (substring `docx-comments-rail` together
    // with `appendChild`). The `w:comment` substring is absent from the current
    // HTML, so the iteration assertion is genuinely red first.
    func test_buildDocxHTML_appendsCardPerComment() {
        let html = buildDocxHTML(
            base64: "UEsDBAoAAAAAAA==",
            jszipScript: "/* stub jszip */",
            docxPreviewScript: "/* stub docx-preview */"
        )
        guard let commentsReadRange = html.range(of: "word/comments.xml") else {
            XCTFail("Expected docx HTML to read the docx package's word/comments.xml")
            return
        }
        guard let mathFallbackRange = html.range(of: "数学公式") else {
            XCTFail("Expected docx HTML to contain the math-formula fallback marker '数学公式'")
            return
        }
        let parsingRegion = String(html[commentsReadRange.upperBound..<mathFallbackRange.lowerBound])
        XCTAssertTrue(
            parsingRegion.contains("w:comment"),
            "Expected the comment-parsing JS (after reading word/comments.xml, before the math fallback) to iterate over each w:comment node"
        )
        XCTAssertTrue(
            parsingRegion.contains("w:t"),
            "Expected the comment-parsing JS to pull each comment's body text (w:t runs)"
        )
        XCTAssertTrue(
            parsingRegion.contains("docx-comments-rail") && parsingRegion.contains("appendChild"),
            "Expected the comment-parsing JS to append a card per comment into the .docx-comments-rail"
        )
    }

    // Acceptance criterion #3 (issue #16): when a `w:comment` carries a
    // `w:author` and a `w:date`, the corresponding card surfaces that author
    // and date. Criterion #2 already builds one card per comment carrying the
    // body text, but the card-building JS reads only the body runs (`w:t`); it
    // never reads the comment element's `w:author` / `w:date` attributes. This
    // anchors on the same comment-parsing region as criterion #2 (between the
    // `word/comments.xml` read and the math-formula fallback marker '数学公式')
    // and asserts the JS in that region reads each comment's `w:author` and
    // `w:date` attributes. Both substrings are absent from the current
    // card-building JS, so the assertions are genuinely red first.
    func test_buildDocxHTML_cardShowsAuthorAndDate() {
        let html = buildDocxHTML(
            base64: "UEsDBAoAAAAAAA==",
            jszipScript: "/* stub jszip */",
            docxPreviewScript: "/* stub docx-preview */"
        )
        guard let commentsReadRange = html.range(of: "word/comments.xml") else {
            XCTFail("Expected docx HTML to read the docx package's word/comments.xml")
            return
        }
        guard let mathFallbackRange = html.range(of: "数学公式") else {
            XCTFail("Expected docx HTML to contain the math-formula fallback marker '数学公式'")
            return
        }
        let parsingRegion = String(html[commentsReadRange.upperBound..<mathFallbackRange.lowerBound])
        XCTAssertTrue(
            parsingRegion.contains("w:author"),
            "Expected the comment-parsing JS to read each comment's w:author attribute so the card can show the author"
        )
        XCTAssertTrue(
            parsingRegion.contains("w:date"),
            "Expected the comment-parsing JS to read each comment's w:date attribute so the card can show the date"
        )
    }

    // Acceptance criterion #4 (issue #16): when `word/comments.xml` is missing
    // or parses to zero `w:comment` nodes, the comment-parsing path must create
    // no card and no `.docx-comments-rail` container. The JS already returns
    // early when the file is absent (`if (!commentsFile) return;`), but the
    // zero-`w:comment` case is only handled implicitly by lazy rail creation
    // inside the iteration loop. This pins an explicit empty-guard: in the
    // comment-parsing region (after the `word/comments.xml` read, before the
    // math-formula fallback marker '数学公式') the JS must early-exit on a
    // zero comment count (substring `comments.length === 0`) before any rail is
    // created. That substring is absent from the current HTML, so the assertion
    // is genuinely red first. Paired with the existing
    // test_buildDocxHTML_noSidebarContainerWhenCommentFree, which pins that no
    // static `<div class="docx-comments-rail">` container is emitted.
    func test_buildDocxHTML_noRailWhenNoComments() {
        let html = buildDocxHTML(
            base64: "UEsDBAoAAAAAAA==",
            jszipScript: "/* stub jszip */",
            docxPreviewScript: "/* stub docx-preview */"
        )
        guard let commentsReadRange = html.range(of: "word/comments.xml") else {
            XCTFail("Expected docx HTML to read the docx package's word/comments.xml")
            return
        }
        guard let mathFallbackRange = html.range(of: "数学公式") else {
            XCTFail("Expected docx HTML to contain the math-formula fallback marker '数学公式'")
            return
        }
        let parsingRegion = String(html[commentsReadRange.upperBound..<mathFallbackRange.lowerBound])
        XCTAssertTrue(
            parsingRegion.contains("comments.length === 0"),
            "Expected the comment-parsing JS to early-exit on a zero w:comment count (comments.length === 0) before any .docx-comments-rail is created"
        )
    }

    // Acceptance criterion #5 (issue #16): the HTML buildDocxHTML produces must
    // still pass `renderChanges: true` to docx-preview, so revisions
    // (`w:ins`/`w:del`) keep rendering exactly as before. This is a regression
    // pin guarding against the comment-rendering change accidentally disturbing
    // the track-changes option. `renderChanges: true` is part of the production
    // HTML today, so this is a staged-red test: the deliberately wrong expected
    // value below fires the assertion red first, then is corrected to current
    // reality in the follow-up commit.
    func test_buildDocxHTML_preservesRenderChanges() {
        let html = buildDocxHTML(
            base64: "UEsDBAoAAAAAAA==",
            jszipScript: "/* stub jszip */",
            docxPreviewScript: "/* stub docx-preview */"
        )
        XCTAssertTrue(
            html.contains("renderChanges: true"),
            "Expected docx HTML to preserve renderChanges: true so revisions (w:ins/w:del) keep rendering"
        )
    }

    // Acceptance criterion #1 (issue #18): each comment card buildDocxHTML
    // produces must carry a `data-comment-id` attribute whose value comes from
    // the w:comment's `w:id`, so the card can be paired with the body anchor by
    // id. Criterion #2 of issue #16 already builds one card per w:comment, but
    // the card-building JS never reads the comment's `w:id` and never sets
    // `data-comment-id` on the card. This anchors on the comment-parsing region
    // (between the `word/comments.xml` read and the math-formula fallback marker
    // '数学公式') and asserts the JS in that region reads each comment's `w:id`
    // attribute and sets it as the card's `data-comment-id`. Both `w:id` and
    // `data-comment-id` are absent from the current card-building JS, so the
    // assertions are genuinely red first.
    func test_buildDocxHTML_cardCarriesCommentId() {
        let html = buildDocxHTML(
            base64: "UEsDBAoAAAAAAA==",
            jszipScript: "/* stub jszip */",
            docxPreviewScript: "/* stub docx-preview */"
        )
        guard let commentsReadRange = html.range(of: "word/comments.xml") else {
            XCTFail("Expected docx HTML to read the docx package's word/comments.xml")
            return
        }
        guard let mathFallbackRange = html.range(of: "数学公式") else {
            XCTFail("Expected docx HTML to contain the math-formula fallback marker '数学公式'")
            return
        }
        let parsingRegion = String(html[commentsReadRange.upperBound..<mathFallbackRange.lowerBound])
        XCTAssertTrue(
            parsingRegion.contains("w:id"),
            "Expected the comment-parsing JS to read each comment's w:id attribute so the card can be paired with the body anchor"
        )
        XCTAssertTrue(
            parsingRegion.contains("data-comment-id"),
            "Expected each comment card to carry a data-comment-id attribute taken from the w:comment's w:id"
        )
    }

    // Acceptance criterion #2 (issue #18): the HTML buildDocxHTML produces must
    // carry a CSS rule, inside its `<style>` block, that gives the body
    // highlight range — the span carrying `data-comment-id` — a visible
    // background. Criterion #3 wraps the body range between commentRangeStart /
    // commentRangeEnd in a `<span data-comment-id="...">`; without a CSS rule
    // that span is invisible. This anchors on the `<style>` block (between
    // `<style>` and `</style>`) and asserts that region carries a selector
    // matching a span by its `data-comment-id` attribute (substring
    // `span[data-comment-id]`) and that the rule sets a visible `background`.
    // Both substrings are absent from the current `<style>` block, so the
    // assertions are genuinely red first.
    func test_buildDocxHTML_highlightSpanHasBackgroundCss() {
        let html = buildDocxHTML(
            base64: "UEsDBAoAAAAAAA==",
            jszipScript: "/* stub jszip */",
            docxPreviewScript: "/* stub docx-preview */"
        )
        guard let styleOpenRange = html.range(of: "<style>") else {
            XCTFail("Expected docx HTML to contain a <style> block")
            return
        }
        guard let styleCloseRange = html.range(of: "</style>", range: styleOpenRange.upperBound..<html.endIndex) else {
            XCTFail("Expected docx HTML's <style> block to be closed with </style>")
            return
        }
        let styleBlock = String(html[styleOpenRange.upperBound..<styleCloseRange.lowerBound])
        guard let selectorRange = styleBlock.range(of: "span[data-comment-id]") else {
            XCTFail("Expected the <style> block to carry a CSS rule selecting the body highlight span by its data-comment-id attribute (span[data-comment-id])")
            return
        }
        let ruleRegion = String(styleBlock[selectorRange.upperBound..<styleBlock.endIndex])
        guard let braceRange = ruleRegion.range(of: "}") else {
            XCTFail("Expected the span[data-comment-id] CSS rule to be closed with }")
            return
        }
        let declarations = String(ruleRegion[ruleRegion.startIndex..<braceRange.lowerBound])
        XCTAssertTrue(
            declarations.contains("background"),
            "Expected the span[data-comment-id] CSS rule to set a visible background on the body highlight range"
        )
    }

    // Acceptance criterion #3 (issue #18): buildDocxHTML's output must contain a
    // piece of JS that walks the docx-preview-rendered DOM, finds the
    // comment-range boundary marker nodes (the comment nodes corresponding to
    // commentRangeStart / commentRangeEnd), and wraps the body text between that
    // marker pair into a highlight <span> carrying data-comment-id. docx-preview
    // renders those markers as DOM comment nodes; the new JS walks #container with
    // a NodeFilter.SHOW_COMMENT TreeWalker to find the pair and wraps the body
    // between them in a <span data-comment-id="...">. This anchors on the normal
    // render-path region (between the word/comments.xml read and the math-formula
    // fallback marker '数学公式', following the established pattern) and asserts
    // the JS in that region mentions both commentRangeStart and commentRangeEnd
    // and builds a span carrying data-comment-id. All three substrings are absent
    // from the current JS, so the assertions are genuinely red first.
    func test_buildDocxHTML_wrapsCommentRangeInSpan() {
        let html = buildDocxHTML(
            base64: "UEsDBAoAAAAAAA==",
            jszipScript: "/* stub jszip */",
            docxPreviewScript: "/* stub docx-preview */"
        )
        guard let commentsReadRange = html.range(of: "word/comments.xml") else {
            XCTFail("Expected docx HTML to read the docx package's word/comments.xml")
            return
        }
        guard let mathFallbackRange = html.range(of: "数学公式") else {
            XCTFail("Expected docx HTML to contain the math-formula fallback marker '数学公式'")
            return
        }
        let normalPathRegion = String(html[commentsReadRange.upperBound..<mathFallbackRange.lowerBound])
        XCTAssertTrue(
            normalPathRegion.contains("commentRangeStart"),
            "Expected the normal-path JS to find the commentRangeStart boundary marker node so it knows where the annotated body range begins"
        )
        XCTAssertTrue(
            normalPathRegion.contains("commentRangeEnd"),
            "Expected the normal-path JS to find the commentRangeEnd boundary marker node so it knows where the annotated body range ends"
        )
        XCTAssertTrue(
            normalPathRegion.contains("data-comment-id"),
            "Expected the normal-path JS to wrap the body between the marker pair in a <span data-comment-id=\"...\"> so the annotated passage gets a visible highlight"
        )
    }

    // Acceptance criterion #4 (issue #18): buildDocxHTML's output must contain a
    // piece of JS that vertically positions each comment card by its paired body
    // highlight span. Criterion #1 gave each card a `data-comment-id`; criterion
    // #3 gave each body highlight span the same `data-comment-id`. Once the span
    // is wrapped it has a real `offsetTop` in the page, so the positioning JS
    // reads the paired span's `offsetTop` and writes it into the card's vertical
    // position via `top` or `transform: translateY(...)`. This anchors on the
    // normal render-path region (between the word/comments.xml read and the
    // math-formula fallback marker '数学公式', following the established pattern)
    // and asserts the JS in that region reads `offsetTop` and writes either `top`
    // or `transform: translateY`. Both substrings are absent from the current JS,
    // so the assertions are genuinely red first.
    func test_buildDocxHTML_positionsCardByOffsetTop() {
        let html = buildDocxHTML(
            base64: "UEsDBAoAAAAAAA==",
            jszipScript: "/* stub jszip */",
            docxPreviewScript: "/* stub docx-preview */"
        )
        guard let commentsReadRange = html.range(of: "word/comments.xml") else {
            XCTFail("Expected docx HTML to read the docx package's word/comments.xml")
            return
        }
        guard let mathFallbackRange = html.range(of: "数学公式") else {
            XCTFail("Expected docx HTML to contain the math-formula fallback marker '数学公式'")
            return
        }
        let normalPathRegion = String(html[commentsReadRange.upperBound..<mathFallbackRange.lowerBound])
        XCTAssertTrue(
            normalPathRegion.contains("offsetTop"),
            "Expected the normal-path JS to read each card's paired body highlight span's offsetTop so the card can be positioned to its annotated passage"
        )
        XCTAssertTrue(
            normalPathRegion.contains("transform = 'translateY") ||
            normalPathRegion.contains("transform: translateY") ||
            normalPathRegion.contains(".style.top"),
            "Expected the normal-path JS to write the paired span's offsetTop into the card's vertical position via top or transform: translateY"
        )
    }

    // Acceptance criterion #5 (issue #18): buildDocxHTML's output must contain a
    // piece of JS that binds a click listener to each comment card, and whose
    // handler scrolls the page to the paired body highlight span. Criterion #1
    // gave each card a `data-comment-id`; criterion #3 gave each body highlight
    // span the same `data-comment-id`. Clicking a card should now jump to the
    // passage it annotates: the JS registers a click listener
    // (`addEventListener` with `'click'`) on each card, and the handler looks up
    // the paired span by its `data-comment-id` and `scrollIntoView`s to it. This
    // anchors on the normal render-path region (between the word/comments.xml
    // read and the math-formula fallback marker '数学公式', following the
    // established pattern) and asserts the JS in that region registers a click
    // listener and calls scrollIntoView. Both substrings are absent from the
    // current JS, so the assertions are genuinely red first.
    func test_buildDocxHTML_cardClickScrollsToHighlight() {
        let html = buildDocxHTML(
            base64: "UEsDBAoAAAAAAA==",
            jszipScript: "/* stub jszip */",
            docxPreviewScript: "/* stub docx-preview */"
        )
        guard let commentsReadRange = html.range(of: "word/comments.xml") else {
            XCTFail("Expected docx HTML to read the docx package's word/comments.xml")
            return
        }
        guard let mathFallbackRange = html.range(of: "数学公式") else {
            XCTFail("Expected docx HTML to contain the math-formula fallback marker '数学公式'")
            return
        }
        let normalPathRegion = String(html[commentsReadRange.upperBound..<mathFallbackRange.lowerBound])
        XCTAssertTrue(
            normalPathRegion.contains("addEventListener") &&
            normalPathRegion.contains("'click'"),
            "Expected the normal-path JS to bind a click listener to each comment card so clicking it jumps to the annotated passage"
        )
        XCTAssertTrue(
            normalPathRegion.contains("scrollIntoView"),
            "Expected the card's click handler to find the paired body highlight span by its data-comment-id and scrollIntoView to it"
        )
    }

    // Acceptance criterion #6 (issue #18): buildDocxHTML's output must contain a
    // piece of JS that binds a click listener to each body highlight span, and
    // whose handler locates/emphasizes the comment card with the same
    // data-comment-id. Criterion #5 made the card -> body direction work
    // (clicking a card scrolls to its highlight span); this is the reverse
    // direction: clicking a highlight span finds the paired card by its
    // data-comment-id, scrolls to it, and adds an emphasis class
    // (`classList.add`). The current JS only binds the card-side click handler,
    // so the span-side handler and the emphasis-class call are absent. This
    // anchors on the normal render-path region (between the word/comments.xml
    // read and the math-formula fallback marker '数学公式', following the
    // established pattern) and asserts the JS in that region wires the highlight
    // span to a click handler that scrolls to the paired card and adds an
    // emphasis class via classList.add. The classList.add substring is absent
    // from the current JS, so the assertion is genuinely red first.
    func test_buildDocxHTML_highlightClickEmphasizesCard() {
        let html = buildDocxHTML(
            base64: "UEsDBAoAAAAAAA==",
            jszipScript: "/* stub jszip */",
            docxPreviewScript: "/* stub docx-preview */"
        )
        guard let commentsReadRange = html.range(of: "word/comments.xml") else {
            XCTFail("Expected docx HTML to read the docx package's word/comments.xml")
            return
        }
        guard let mathFallbackRange = html.range(of: "数学公式") else {
            XCTFail("Expected docx HTML to contain the math-formula fallback marker '数学公式'")
            return
        }
        let normalPathRegion = String(html[commentsReadRange.upperBound..<mathFallbackRange.lowerBound])
        XCTAssertTrue(
            normalPathRegion.contains("span.addEventListener") &&
            normalPathRegion.contains("'click'"),
            "Expected the normal-path JS to bind a click listener to each body highlight span so clicking the passage jumps to its comment card"
        )
        XCTAssertTrue(
            normalPathRegion.contains("classList.add"),
            "Expected the highlight span's click handler to find the paired comment card by its data-comment-id, scroll to it, and add an emphasis class via classList.add"
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
