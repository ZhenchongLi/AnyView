### Claude

## Verdict
changes-requested

## Real issues

1. Criterion 6 is still half-covered. The criterion says "Two unit tests assert the absence of the sidebar container for a comment-free input on each path." Only the docx half exists: `test_buildDocxHTML_noSidebarContainerWhenCommentFree` (`Tests/AnyViewAppTests/WordCommentHTMLTests.swift:42`). No test feeds `transformDocmodComments` an input with no `<aside data-type="comments">` block and asserts the output carries no `docmod-comments-rail` container. The behavior is in the code — `transformDocmodComments` returns the input unchanged when `rangeOfCommentsAside` finds nothing (`Sources/AnyViewApp/WordCommentHTML.swift:154-156`) — but the criterion names two tests and one is missing. A later edit that injects an empty rail on the docmod path passes `swift test` untouched. Last cycle flagged this same gap; criterion 7 got its test this round, this one did not.

2. A `.doct` with comments renders broken chrome. `transformDocmodComments` injects `.docmod-comments-rail { position: fixed; top: 0; right: 0; bottom: 0; ... }` and `body { margin-right: 280px; }` inside the `<aside>` (`Sources/AnyViewApp/WordCommentHTML.swift:176-188`). On the doct path that `<aside>` gets sliced out of `<body>`/`</body>` and dropped into `buildDoctHtml`'s `.preview-frame` (`WebRenderer.swift:1659-1664`, `:1783`). The fixed rail then pins to the full viewport and covers the doct header and metadata sections, and `body { margin-right: 280px }` shifts the entire doct page, not the preview. The CSS that scopes cleanly to a standalone `.docmod` page leaks into and overlays the `.doct` viewer. Criterion 4's "right-side sidebar" placement is wrong on the doct path. Fires whenever a `.doct` carries a comment.

3. The docx error path lost behavior in the move to `buildDocxHTML`. Trunk's inline string showed the exception detail on render failure — `'渲染失败: ' + (e2 && e2.message ? e2.message : e2)` — and cleared the math-fallback banner after 3 seconds with `setTimeout(... 3000)`. `buildDocxHTML` now shows a fixed `'无法渲染文档'` with no detail and leaves the `'数学公式已转为占位符'` banner pinned forever (`Sources/AnyViewApp/WordCommentHTML.swift:121,124`). A user who hits a render failure loses the error text, and a user whose doc needed the math fallback keeps a stale banner on screen. This is a regression slipped into a refactor advertised as a pure extraction.

## Questions

- The `<aside>`-survives-the-slice claim rests on the design doc, not a fixture from real docmod 2.15.23 read output. `test_transformDocmodComments_placesCommentInSidebar` feeds a hand-written input string. Confirm the real `read` envelope's `html` field places `<aside data-type="comments">` inside `<body>` so the doct slice keeps it.
- `DocmodCLI.render` is now unreferenced by the production docx/docmod/doct read paths. Keep it or drop it?

## Nits

- `test_transformDocmodComments_placesCommentInSidebar` asserts `output.contains("cm1")` (`WordCommentHTMLTests.swift:138-141`). The input already contains `<mark data-id="cm1">`, which the transform preserves untouched, so this assertion passes even if the card carries no id. The code does put `data-comment-id="cm1"` on the card (`WordCommentHTML.swift:166`); the test just doesn't pin it. Assert on `data-comment-id="cm1"` to make the anchor association real.
- `AGENTS.md:30` still states "There is no Swift test target, so `swift test` will fail." That is now false. Update it.

## Functional evidence
- Criterion 1 — pass: `swift test` builds `AnyViewAppTests` and runs 9 tests, "Executed 9 tests, with 0 failures (0 unexpected)", exit zero. `.testTarget(name: "AnyViewAppTests", dependencies: ["AnyViewApp"], path: "Tests/AnyViewAppTests")` at `Package.swift:17-21`.
- Criterion 2 — pass: `buildDocxHTML(base64:jszipScript:docxPreviewScript:) -> String` at `Sources/AnyViewApp/WordCommentHTML.swift:9`, no `WebRenderer`/`NSView`/`WKWebView`; `test_buildDocxHTML_isCallableStandalone` calls it with stub scripts and asserts non-empty. Wired into production at `WebRenderer.swift:1480`.
- Criterion 3 — pass: `.docx-comments-rail` CSS at `WordCommentHTML.swift:51,57`, `moveCommentNodesToRail` JS hook at `:70-83` (called at `:101`, `:120`); `test_buildDocxHTML_containsCommentSidebarScaffold` asserts both `.docx-comments-rail` and `moveCommentNodesToRail` present.
- Criterion 4 — pass: `transformDocmodComments(html:)` at `WordCommentHTML.swift:153` parses the `<aside data-type="comments">` block and rebuilds each comment as a `docmod-comment-card` with `data-comment-id="cm1"` inside a `docmod-comments-rail`; `test_transformDocmodComments_placesCommentInSidebar` (`WordCommentHTMLTests.swift:108`) feeds a fixed input with `data-id="cm1"`, author `AI`, text "Consider rewording this phrase.", and `<mark data-id="cm1">`, asserting rail, author, text, and cm1 all present. Placement is correct on the standalone docmod page; broken on the doct path (Real issue 2).
- Criterion 5 — pass: `DocmodCLI.docmodReadArguments(path:)` returns `["read", path]` at `DocmodCLI.swift:70`; `test_docmodReadArguments_notRender` asserts not `["render", path]` and first element `"read"`; `test_loadDocmodAndDoctContent_useReadNotRender` confirms `loadDocmodContent` (`WebRenderer.swift:1629`) and `loadDoctContent` (`WebRenderer.swift:1660`) call `DocmodCLI.readHTML`, not `DocmodCLI.render`.
- Criterion 6 — fail: only the docx half is tested. `test_buildDocxHTML_noSidebarContainerWhenCommentFree` (`WordCommentHTMLTests.swift:42`) covers the docx path. No test covers the docmod path — no `test_transformDocmodComments_noSidebarWhenNoAside` feeding an aside-free input and asserting no `docmod-comments-rail` container. The criterion requires two tests; one exists.
- Criterion 7 — pass: `test_buildDocxHTML_preservesRenderAsyncAndCJKFonts` (`WordCommentHTMLTests.swift:60`) asserts `buildDocxHTML`'s output contains `docx.renderAsync` and `font-family: 'SimSun'`. Both substrings present at `WordCommentHTML.swift:100` and `:31`.
