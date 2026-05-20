### Claude

## Verdict
changes-requested

## Real issues

1. The docmod and doct paths still call `DocmodCLI.render`, which strips comments. `loadDocmodContent` (`WebRenderer.swift:1629`) and `loadDoctContent` (`WebRenderer.swift:1658`) both run `["render", filePath]`. Open a `.docmod` or `.doct` file with one comment today: the author and text never reach the web view. `DocmodCLI.docmodReadArguments(path:)` exists (`DocmodCLI.swift:70`) but no production code calls it. Grep for `docmodReadArguments` returns only the function definition and its test. The function is dead. Criterion 5's behavioral half ships zero behavior.

2. No `transformDocmodComments(html:)` function exists. Grep across the worktree returns hits only in `design-9.md` and this file. The docmod side of #9 has no sidebar at all — no rail, no card, nothing. Criterion 4 ships zero behavior.

3. Criterion 7 has no test. `buildDocxHTML` carries the `docx.renderAsync` call (`WordCommentHTML.swift:100`) and the CJK `@font-face` block (`WordCommentHTML.swift:29-48`), but no test pins either substring. The next edit to that string can drop the SimSun mapping or break renderAsync and `swift test` stays green. The criterion demands a test asserting both remain; it is missing.

4. Criterion 6 has no test on either path. No `test_buildDocxHTML_noSidebarContainerWhenCommentFree`, no `test_transformDocmodComments_noSidebarWhenNoAside`. The "zero comments means no empty rail" guarantee is unverified for both docx and docmod.

## Questions

- The docx blockers from earlier rounds are closed: `loadDocxContent` routes through `buildDocxHTML` (`WebRenderer.swift:1480`), the divergent inline copy is gone, and `test_loadDocxContent_delegatesToBuildDocxHTML` pins the wiring. Confirmed green.
- `docmod read` returns a JSON envelope (`status`, `command`, `input`, `html`, `summary`), not raw HTML. When the read path lands: where does the JSON parse live, and what happens on a malformed envelope — fall back to `render`, or surface an error?
- `loadDoctContent` slices `<body>...</body>` out of the render output (`WebRenderer.swift:1659-1662`). The design claims read output also has a `<body>` with the `<aside>` inside it after `</article>`, so the slice survives. Checked against real docmod 2.15.23 read output, or still an assumption?
- The docx zero-comment guarantee (criterion 6) is a runtime JS property; `swift test` cannot run the rail-building JS. The static-markup test confirms no pre-rendered container ships, but the real "no empty rail" check happens only in the browser. How is that verified before merge — a manual run against a comment-free docx?

## Nits

None.

## Functional evidence
- Criterion 1 — pass: `swift test` from the package root builds `AnyViewAppTests` and runs 5 tests. Output: "Executed 5 tests, with 0 failures (0 unexpected)"; run exits zero. `.testTarget(name: "AnyViewAppTests", dependencies: ["AnyViewApp"], path: "Tests/AnyViewAppTests")` present in `Package.swift:17-21`.
- Criterion 2 — pass: `buildDocxHTML(base64:jszipScript:docxPreviewScript:) -> String` in `WordCommentHTML.swift:9` takes no `WebRenderer`/`NSView`/`WKWebView`. `test_buildDocxHTML_isCallableStandalone` calls it with stub scripts and asserts a non-empty `String`; passes. Wired into production: `loadDocxContent` calls it at `WebRenderer.swift:1480`.
- Criterion 3 — pass: `buildDocxHTML` contains the rail CSS selector `.docx-comments-rail` (`WordCommentHTML.swift:51,57`) and the JS hook `moveCommentNodesToRail` that queries docx-preview comment nodes and moves them into the rail (`WordCommentHTML.swift:70-83`, called at lines 101 and 120). `test_buildDocxHTML_containsCommentSidebarScaffold` asserts both substrings present; passes.
- Criterion 4 — fail: no `transformDocmodComments(html:)` function. Grep returns hits only in `design-9.md` and `review-notes-9.md`. No fixed-input test placing a `cm1` comment from author `AI` into a sidebar.
- Criterion 5 — fail: `DocmodCLI.docmodReadArguments(path:)` exists (`DocmodCLI.swift:70`) and `test_docmodReadArguments_notRender` asserts it is not `["render", path]` and starts with `"read"` — both pass. But production never calls it; `loadDocmodContent` (`WebRenderer.swift:1629`) and `loadDoctContent` (`WebRenderer.swift:1658`) still call `DocmodCLI.render`. AnyView does not obtain comment-bearing HTML from a non-`render` invocation. The behavioral half of the criterion is unmet.
- Criterion 6 — fail: neither zero-comment test exists. No `test_buildDocxHTML_noSidebarContainerWhenCommentFree`, no `test_transformDocmodComments_noSidebarWhenNoAside`.
- Criterion 7 — fail: no `test_buildDocxHTML_preservesRenderAsyncAndCJKFonts`. The `docx.renderAsync` call (`WordCommentHTML.swift:100`) and the CJK `@font-face` block (`WordCommentHTML.swift:29-48`) are present, but no test asserts both substrings remain.
