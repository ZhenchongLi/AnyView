### Claude

## Verdict
changes-requested

## Real issues

1. Criterion 6 is still half-covered. The criterion names two tests: "Two unit tests assert the absence of the sidebar container for a comment-free input on each path." Only the docx half exists — `test_buildDocxHTML_noSidebarContainerWhenCommentFree` (`Tests/AnyViewAppTests/WordCommentHTMLTests.swift:42`). No test feeds `transformDocmodComments` an input with no `<aside data-type="comments">` block and asserts the output carries no `docmod-comments-rail` container. The behavior is in the code — `transformDocmodComments` returns the input unchanged when `rangeOfCommentsAside` finds nothing (`Sources/AnyViewApp/WordCommentHTML.swift:154-156`) — but the criterion demands two tests and one is absent. A later edit that injects an empty rail on the docmod path passes `swift test` untouched. Three cycles have flagged this same gap. The docx half landed, the docmod half never did.

2. The docx error path lost behavior in the move to `buildDocxHTML`, and the move was sold as a pure extraction. Trunk's inline string showed the exception detail on render failure — `'渲染失败: ' + (e2 && e2.message ? e2.message : e2)` (`origin/main` `WebRenderer.swift:1566`) — and cleared the math-fallback banner after 3 seconds with `setTimeout(... 3000)` (`origin/main` `WebRenderer.swift:1563`). `buildDocxHTML` now shows a fixed `'无法渲染文档'` with no detail and leaves the `'数学公式已转为占位符'` banner pinned forever (`Sources/AnyViewApp/WordCommentHTML.swift:121,124`). A user who hits a render failure loses the error text. A user whose doc needed the math fallback keeps a stale banner on screen for the life of the view. Two user-visible regressions inside a refactor that should have changed nothing but where the HTML lives.

## Questions

- Real issue 2 from the prior cycle (doct chrome) is fixed. The docmod rail is now `float: right; width: 280px` with no `position: fixed` and no global `body` selector (`Sources/AnyViewApp/WordCommentHTML.swift:181`), and `test_transformDocmodComments_doesNotShiftEmbeddingBody` (`WordCommentHTMLTests.swift:152`) pins it. No further action.
- The `<aside>`-survives-the-slice claim still rests on the design doc, not a fixture from real docmod 2.15.23 read output. `test_transformDocmodComments_placesCommentInSidebar` feeds a hand-written input string. Confirm the real `read` envelope's `html` field places `<aside data-type="comments">` inside `<body>` so the doct slice (`WebRenderer.swift:1659-1664`) keeps it.
- `DocmodCLI.render` is now unreferenced by the production docx/docmod/doct read paths. Keep it or drop it?

## Nits

- `test_transformDocmodComments_placesCommentInSidebar` asserts `output.contains("cm1")` (`WordCommentHTMLTests.swift:138-141`). The input already contains `<mark data-id="cm1">`, which the transform preserves untouched, so this assertion passes even if the card carries no id. The code does put `data-comment-id="cm1"` on the card (`WordCommentHTML.swift:166`); the test just doesn't pin it. Assert on `data-comment-id="cm1"` to make the anchor association real.
- `AGENTS.md:30` still states "There is no Swift test target, so `swift test` will fail." That is now false. Update it.

## Functional evidence
- Criterion 1 — pass: `swift test` builds `AnyViewAppTests` and runs 10 tests, "Executed 10 tests, with 0 failures (0 unexpected)", exit zero. `.testTarget(name: "AnyViewAppTests", dependencies: ["AnyViewApp"], path: "Tests/AnyViewAppTests")` at `Package.swift:17-21`.
- Criterion 2 — pass: `buildDocxHTML(base64:jszipScript:docxPreviewScript:) -> String` at `Sources/AnyViewApp/WordCommentHTML.swift:9`, no `WebRenderer`/`NSView`/`WKWebView`; `test_buildDocxHTML_isCallableStandalone` calls it with stub scripts and asserts non-empty. Wired into production at `WebRenderer.swift:1480`.
- Criterion 3 — pass: `.docx-comments-rail` CSS at `WordCommentHTML.swift:51,57`, `moveCommentNodesToRail` JS hook at `:70-83` (called at `:101`, `:120`); `test_buildDocxHTML_containsCommentSidebarScaffold` asserts both `.docx-comments-rail` and `moveCommentNodesToRail` present.
- Criterion 4 — pass: `transformDocmodComments(html:)` at `WordCommentHTML.swift:153` parses the `<aside data-type="comments">` block and rebuilds each comment as a `docmod-comment-card` with `data-comment-id="cm1"` inside a `docmod-comments-rail`; `test_transformDocmodComments_placesCommentInSidebar` (`WordCommentHTMLTests.swift:108`) feeds a fixed input with `data-id="cm1"`, author `AI`, text "Consider rewording this phrase.", and `<mark data-id="cm1">`, asserting rail, author, text, and cm1 all present. Rail CSS is self-scoped (`float: right`, no global `body` rule), so the doct slice no longer breaks doct chrome.
- Criterion 5 — pass: `DocmodCLI.docmodReadArguments(path:)` returns `["read", path]` at `DocmodCLI.swift:70`; `test_docmodReadArguments_notRender` asserts not `["render", path]` and first element `"read"`; `test_loadDocmodAndDoctContent_useReadNotRender` confirms `loadDocmodContent` (`WebRenderer.swift:1629`) and `loadDoctContent` (`WebRenderer.swift:1660`) call `DocmodCLI.readHTML`, not `DocmodCLI.render`.
- Criterion 6 — fail: only the docx half is tested. `test_buildDocxHTML_noSidebarContainerWhenCommentFree` (`WordCommentHTMLTests.swift:42`) covers the docx path. No test covers the docmod path — no `test_transformDocmodComments_noSidebarWhenNoAside` feeding an aside-free input and asserting no `docmod-comments-rail` container. The criterion requires two tests; one exists.
- Criterion 7 — pass: `test_buildDocxHTML_preservesRenderAsyncAndCJKFonts` (`WordCommentHTMLTests.swift:60`) asserts `buildDocxHTML`'s output contains `docx.renderAsync` and `font-family: 'SimSun'`. Both substrings present at `WordCommentHTML.swift:100` and `:31`.
