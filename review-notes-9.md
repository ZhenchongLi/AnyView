### Claude

## Verdict
changes-requested

## Real issues

1. The comment feature does not exist yet. The whole point of issue #9 is showing Word comments in a right-side sidebar. No code does that. No `.docx-comments-rail` CSS anywhere. No JavaScript moves `docx-preview` comment nodes into a rail. No `transformDocmodComments`. No `docmodReadArguments`. The docx and docmod/doct paths render exactly as they did on `main`. Open a commented Word file today and you see no comments. Criteria 3, 4, 5, 6 ship zero code.

2. `buildDocxHTML` is dead code that already drifts from production. `loadDocxContent` (`Sources/AnyViewApp/WebRenderer.swift:1473`) still builds its own inline HTML string and never calls `buildDocxHTML`. The two strings already differ: the inline version sets the error path to `'渲染失败: ' + ...` and adds a `setTimeout` reset (`WebRenderer.swift:1563-1566`); `buildDocxHTML` says `'无法渲染文档'` and has no timeout (`WordCommentHTML.swift:92-95`). A test asserting on `buildDocxHTML` proves nothing about what the user sees, because the web view loads the inline copy. Wire `loadDocxContent` to call `buildDocxHTML` and delete the inline copy, or the function is theater.

3. `buildDocxHTML` carries no comment sidebar scaffold. Criterion 3 requires the returned string to contain a right-rail CSS selector and the JS hook that moves comment nodes into it. The function has neither. The CSS block stops at the font-face rules; the JS does `renderAsync` and nothing about comments past `renderComments: true`, which the design itself says produces no visible sidebar.

4. The docmod and doct paths still call `DocmodCLI.render`, which runs `["render", filePath]` (`DocmodCLI.swift:68,75`). The design states `docmod render` strips the comment text and the `<aside>` block. No `read` path, no JSON-envelope parse, no `html`-field extraction exists. Comments stay invisible on `.docmod` and `.doct`.

5. Six of the seven named tests from the design are absent. `WordCommentHTMLTests.swift` holds two: `test_testTargetRuns` and `test_buildDocxHTML_isCallableStandalone`. Missing: `test_buildDocxHTML_containsCommentSidebarScaffold`, `test_transformDocmodComments_placesCommentInSidebar`, `test_docmodReadArguments_notRender`, `test_buildDocxHTML_noSidebarContainerWhenCommentFree`, `test_transformDocmodComments_noSidebarWhenNoAside`, `test_buildDocxHTML_preservesRenderAsyncAndCJKFonts`. The suite asserts nothing about the feature.

## Questions

- `docmod read` returns a JSON envelope (`status`, `command`, `input`, `html`, `summary`), not raw HTML. When the read path lands, where does the parse live, and what happens on a malformed envelope — fall back to `render`, or surface an error?
- The docx zero-comment guarantee is a runtime JS property and `swift test` cannot run JS. The static-markup test confirms no pre-rendered container ships, but the real "no empty rail" check for docx happens only in the browser. How is that verified before merge — a manual run against a comment-free docx?

## Nits

None.

## Functional evidence
- Criterion 1 — pass: `swift test` from the package root builds `AnyViewAppTests` and runs two tests. Output: "Executed 2 tests, with 0 failures"; run exits zero. `.testTarget(name: "AnyViewAppTests", dependencies: ["AnyViewApp"], path: "Tests/AnyViewAppTests")` present in `Package.swift`.
- Criterion 2 — pass: `buildDocxHTML(base64:jszipScript:docxPreviewScript:) -> String` exists in `Sources/AnyViewApp/WordCommentHTML.swift:9`, takes no `WebRenderer`/`NSView`/`WKWebView`. `test_buildDocxHTML_isCallableStandalone` calls it with stub scripts and asserts a non-empty `String`. Caveat: the function is not wired into `loadDocxContent`, so it does not produce the HTML the web view actually loads — see Real issue 2.
- Criterion 3 — fail: `buildDocxHTML` contains no `.docx-comments-rail` CSS and no JS hook moving `docx-preview` comment nodes into a rail. Grep for `docx-comments-rail` across `Sources/` returns nothing. No `test_buildDocxHTML_containsCommentSidebarScaffold`.
- Criterion 4 — fail: No `transformDocmodComments(html:)` exists. Grep returns nothing. No fixed-input test placing a `cm1` comment from author `AI` into a sidebar.
- Criterion 5 — fail: No `docmodReadArguments(path:)` exists. `DocmodCLI.render` still runs `["render", filePath]` (`DocmodCLI.swift:75`); both docmod/doct paths still call it. No test asserts the argument list is not `["render", <path>]`.
- Criterion 6 — fail: Neither zero-comment test exists. No `test_buildDocxHTML_noSidebarContainerWhenCommentFree`, no `test_transformDocmodComments_noSidebarWhenNoAside`. The transform they would exercise does not exist.
- Criterion 7 — fail: No `test_buildDocxHTML_preservesRenderAsyncAndCJKFonts`. The `docx.renderAsync` call and CJK `@font-face` block are present in `buildDocxHTML` (`WordCommentHTML.swift:29-48,73`), but no test asserts both substrings remain, and the production path still uses the inline copy regardless.
