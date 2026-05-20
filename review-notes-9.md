### Claude

## Verdict
changes-requested

## Real issues

1. `buildDocxHTML` is dead code. `loadDocxContent` (`Sources/AnyViewApp/WebRenderer.swift:1473`) still builds its own inline HTML string (lines 1480-1573) and hands that to the web view. Nothing calls `buildDocxHTML`. Grep `buildDocxHTML` across `Sources/` returns one hit: the definition. So the comment rail and the `moveCommentNodesToRail` hook never reach a `WKWebView`. Open a commented docx today and you see no comments. The whole point of #9 ships zero user-visible behavior on the docx path.

2. The inline copy already drifted from `buildDocxHTML`, so the function and production diverge before they ever merge. The inline `catch` writes `'渲染失败: ' + (e2 && ...)` and resets the math-placeholder status with `setTimeout` (`WebRenderer.swift:1563,1566`). `buildDocxHTML` writes `'无法渲染文档'` and has no `setTimeout` (`WordCommentHTML.swift:94,121` region). Two strings claiming to be the docx view, neither agreeing. `test_buildDocxHTML_containsCommentSidebarScaffold` passes against a string the user never sees. Wire `loadDocxContent` to call `buildDocxHTML(base64:jszipScript:docxPreviewScript:)` and delete the inline copy, or the test guards nothing.

## Questions

- Criteria 4-7 are not in this diff (per the dispatch note, expected for this cycle). When the docmod `read` path lands: `docmod read` returns a JSON envelope (`status`, `command`, `input`, `html`, `summary`), not raw HTML. Where does the parse live, and what happens on a malformed envelope — fall back to `render`, or surface an error?
- The docx zero-comment guarantee (criterion 6) is a runtime JS property and `swift test` cannot run JS. The static-markup test will confirm no pre-rendered container ships, but the real "no empty rail" check for docx happens only in the browser. How is that verified before merge — a manual run against a comment-free docx?

## Nits

- `buildDocxHTML` duplicates the entire ~90-line inline block from `loadDocxContent` instead of `loadDocxContent` delegating to it. Until the wire-up in Real issue 1 happens, every future docx tweak must be made twice. This is the cause of Real issue 2; collapsing to one source kills both.

## Functional evidence
- Criterion 1 — pass: `swift test` from the package root builds `AnyViewAppTests` and runs 3 tests. Output: "Executed 3 tests, with 0 failures (0 unexpected)"; run exits zero. `.testTarget(name: "AnyViewAppTests", dependencies: ["AnyViewApp"], path: "Tests/AnyViewAppTests")` present in `Package.swift`.
- Criterion 2 — pass: `buildDocxHTML(base64:jszipScript:docxPreviewScript:) -> String` exists in `Sources/AnyViewApp/WordCommentHTML.swift:9`, takes no `WebRenderer`/`NSView`/`WKWebView`. `test_buildDocxHTML_isCallableStandalone` calls it with stub scripts and asserts a non-empty `String`. Caveat: the function is not wired into `loadDocxContent`, so it does not produce the HTML the web view actually loads — see Real issue 1.
- Criterion 3 — pass: `buildDocxHTML` contains the rail CSS selector `.docx-comments-rail` (`WordCommentHTML.swift:51,57`) and the JS hook `moveCommentNodesToRail` that queries `docx-preview` comment nodes and moves them into the rail (`WordCommentHTML.swift:70-83`, called at lines 101 and 120). `test_buildDocxHTML_containsCommentSidebarScaffold` asserts both substrings present; it passes. Caveat: asserts on the standalone string, not on the inline copy the web view loads — see Real issue 2.
- Criterion 4 — fail: out of scope this cycle. No `transformDocmodComments(html:)` exists. Grep returns nothing. No fixed-input test placing a `cm1` comment from author `AI` into a sidebar.
- Criterion 5 — fail: out of scope this cycle. No `docmodReadArguments(path:)` exists. `DocmodCLI.render` still runs `["render", filePath]` (`DocmodCLI.swift:75`); both docmod/doct paths still call it. No test asserts the argument list is not `["render", <path>]`.
- Criterion 6 — fail: out of scope this cycle. Neither zero-comment test exists. No `test_buildDocxHTML_noSidebarContainerWhenCommentFree`, no `test_transformDocmodComments_noSidebarWhenNoAside`.
- Criterion 7 — fail: out of scope this cycle. No `test_buildDocxHTML_preservesRenderAsyncAndCJKFonts`. The `docx.renderAsync` call and CJK `@font-face` block are present in `buildDocxHTML` (`WordCommentHTML.swift` font block + renderAsync), but no test asserts both substrings remain, and the production path still uses the inline copy regardless.
