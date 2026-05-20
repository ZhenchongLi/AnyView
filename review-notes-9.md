### Claude

## Verdict
changes-requested

## Real issues

1. Criterion 6 still has only one of its two tests. The criterion is explicit: "Two unit tests assert the absence of the sidebar container for a comment-free input on each path." The docx half exists — `test_buildDocxHTML_noSidebarContainerWhenCommentFree` (`Tests/AnyViewAppTests/WordCommentHTMLTests.swift:42`). The docmod half does not. No test feeds `transformDocmodComments` an aside-free input and asserts the output carries no `docmod-comments-rail` container. The behavior is in the code — `transformDocmodComments` returns the input unchanged when `rangeOfCommentsAside` finds nothing (`Sources/AnyViewApp/WordCommentHTML.swift:155-157`) and when the aside holds no parseable comments (`:161-163`) — so a test named `test_transformDocmodComments_noSidebarWhenNoAside` would pass today. It is not written. An edit that injects an empty rail on the docmod path ships green. Four cycles have flagged this same gap and it is still open.

## Questions

- Prior cycle Real issue 2 (docx error path) is fixed. `buildDocxHTML` now surfaces the exception detail on render failure — `'渲染失败: ' + (e2 && e2.message ? e2.message : e2)` (`Sources/AnyViewApp/WordCommentHTML.swift:125`) — and clears the math-fallback banner with `setTimeout(... 3000)` (`:122`). `test_buildDocxHTML_surfacesErrorDetailAndClearsMathFallbackBanner` (`WordCommentHTMLTests.swift:109`) pins both. No further action.
- The `<aside>`-survives-the-doct-slice claim still rests on the design doc, not a fixture from real docmod 2.15.23 `read` output. Every docmod test feeds a hand-written input string. Confirm the real `read` envelope's `html` field places `<aside data-type="comments">` inside `<body>` so the slice at `WebRenderer.swift:1662-1665` keeps it.
- `DocmodCLI.render` (`DocmodCLI.swift:115`) is now unreferenced by the docx/docmod/doct paths. Keep it or drop it?

## Nits

- `test_transformDocmodComments_placesCommentInSidebar` asserts `output.contains("cm1")` (`WordCommentHTMLTests.swift:166-169`). The input already contains `<mark data-id="cm1">`, which the transform preserves untouched, so the assertion passes even if the card carries no id. The code does put `data-comment-id="cm1"` on the card (`WordCommentHTML.swift:167`); the test does not pin it. Assert on `data-comment-id="cm1"` to make the anchor association real.
- `AGENTS.md` still states "There is no Swift test target, so `swift test` will fail." That is now false — `swift test` runs 11 tests and exits zero. Update it.

## Functional evidence
- Criterion 1 — pass: `swift test` builds `AnyViewAppTests` and runs 11 tests, "Executed 11 tests, with 0 failures (0 unexpected)", exit zero. `.testTarget(name: "AnyViewAppTests", dependencies: ["AnyViewApp"], path: "Tests/AnyViewAppTests")` at `Package.swift:17-21`.
- Criterion 2 — pass: `buildDocxHTML(base64:jszipScript:docxPreviewScript:) -> String` at `Sources/AnyViewApp/WordCommentHTML.swift:9`, touches no `WebRenderer`/`NSView`/`WKWebView`; `test_buildDocxHTML_isCallableStandalone` calls it with stub scripts and asserts non-empty. Production wires it at `WebRenderer.swift:1480`.
- Criterion 3 — pass: `.docx-comments-rail` CSS at `WordCommentHTML.swift:51,57`, `moveCommentNodesToRail` JS hook at `:70-83` (called at `:101`, `:120`); `test_buildDocxHTML_containsCommentSidebarScaffold` asserts both `.docx-comments-rail` and `moveCommentNodesToRail` present.
- Criterion 4 — pass: `transformDocmodComments(html:)` at `WordCommentHTML.swift:154` parses the `<aside data-type="comments">` block and rebuilds each comment as a `docmod-comment-card` with `data-comment-id="cm1"` inside a `docmod-comments-rail`; `test_transformDocmodComments_placesCommentInSidebar` (`WordCommentHTMLTests.swift:136`) feeds a fixed input with `data-id="cm1"`, author `AI`, text "Consider rewording this phrase.", and `<mark data-id="cm1">`, asserting rail, author, text, and cm1 all present.
- Criterion 5 — pass: `DocmodCLI.docmodReadArguments(path:)` returns `["read", path]` at `DocmodCLI.swift:70`; `test_docmodReadArguments_notRender` asserts not `["render", path]` and first element `"read"`; `test_loadDocmodAndDoctContent_useReadNotRender` confirms `loadDocmodContent` (`WebRenderer.swift:1629`) and `loadDoctContent` (`WebRenderer.swift:1659-1661`) call `DocmodCLI.readHTML`, not `DocmodCLI.render`.
- Criterion 6 — fail: only the docx half is tested. `test_buildDocxHTML_noSidebarContainerWhenCommentFree` (`WordCommentHTMLTests.swift:42`) covers the docx path. No test covers the docmod path — no test feeds `transformDocmodComments` an aside-free input and asserts no `docmod-comments-rail` container. The criterion requires two tests; one exists.
- Criterion 7 — pass: `test_buildDocxHTML_preservesRenderAsyncAndCJKFonts` (`WordCommentHTMLTests.swift:60`) asserts `buildDocxHTML`'s output contains `docx.renderAsync` and `font-family: 'SimSun'`. Both substrings present at `WordCommentHTML.swift:100` and `:31`.
