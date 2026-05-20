### Claude

## Verdict
approve

## Real issues

None.

## Questions

- The `<aside>`-survives-the-doct-slice claim still rests on the design doc, not a fixture from real docmod 2.15.23 `read` output. Every docmod test feeds a hand-written input string. Confirm the real `read` envelope's `html` field puts `<aside data-type="comments">` inside `<body>` so the slice at `WebRenderer.swift:1662-1665` keeps it. If `read` ever wraps the aside outside `<body>`, the doct path drops the sidebar with no failing test.
- `DocmodCLI.render` (`DocmodCLI.swift:115`) is now unreferenced by the docx/docmod/doct paths. Keep it or drop it?

## Nits

- `test_transformDocmodComments_placesCommentInSidebar` asserts `output.contains("cm1")` (`WordCommentHTMLTests.swift:166-169`). The input already carries `<mark data-id="cm1">`, which the transform preserves untouched, so the assertion passes even if the card carried no id. The code does put `data-comment-id="cm1"` on the card (`WordCommentHTML.swift:167`); the test does not pin it. Assert on `data-comment-id="cm1"` to make the anchor association real.
- `AGENTS.md:30` still states "There is no Swift test target, so `swift test` will fail." That is false now — `swift test` runs 12 tests and exits zero. Update it.

## Functional evidence
- Criterion 1 — pass: `swift test` builds `AnyViewAppTests` and runs 12 tests, "Executed 12 tests, with 0 failures (0 unexpected)", exit zero. `.testTarget(name: "AnyViewAppTests", dependencies: ["AnyViewApp"], path: "Tests/AnyViewAppTests")` at `Package.swift:17-21`.
- Criterion 2 — pass: `buildDocxHTML(base64:jszipScript:docxPreviewScript:) -> String` at `Sources/AnyViewApp/WordCommentHTML.swift:9` touches no `WebRenderer`/`NSView`/`WKWebView`; `test_buildDocxHTML_isCallableStandalone` (`WordCommentHTMLTests.swift:9`) calls it with stub scripts and asserts non-empty. Production wires it at `WebRenderer.swift:1480`.
- Criterion 3 — pass: `.docx-comments-rail` CSS at `WordCommentHTML.swift:51,57`; `moveCommentNodesToRail` JS hook at `:70-83` (called at `:101`, `:120`); `test_buildDocxHTML_containsCommentSidebarScaffold` (`WordCommentHTMLTests.swift:18`) asserts both `.docx-comments-rail` and `moveCommentNodesToRail` present.
- Criterion 4 — pass: `transformDocmodComments(html:)` at `WordCommentHTML.swift:154` parses the `<aside data-type="comments">` block and rebuilds each comment as a `docmod-comment-card` with `data-comment-id="cm1"` inside a `docmod-comments-rail`; `test_transformDocmodComments_placesCommentInSidebar` (`WordCommentHTMLTests.swift:136`) feeds a fixed input with `data-id="cm1"`, author `AI`, text "Consider rewording this phrase.", and `<mark data-id="cm1">`, asserting rail, author, text, and cm1 all present.
- Criterion 5 — pass: `DocmodCLI.docmodReadArguments(path:)` returns `["read", path]` at `DocmodCLI.swift:70`; `test_docmodReadArguments_notRender` (`WordCommentHTMLTests.swift:233`) asserts not `["render", path]` and first element `"read"`; `test_loadDocmodAndDoctContent_useReadNotRender` (`WordCommentHTMLTests.swift:255`) confirms `loadDocmodContent` (`WebRenderer.swift:1629`) and `loadDoctContent` (`WebRenderer.swift:1659-1661`) call `DocmodCLI.readHTML`, not `DocmodCLI.render`.
- Criterion 6 — pass: both halves tested. Docx: `test_buildDocxHTML_noSidebarContainerWhenCommentFree` (`WordCommentHTMLTests.swift:42`) asserts the comment-free build has no `<div class="docx-comments-rail">` container. Docmod: `test_transformDocmodComments_noSidebarWhenNoAside` (`WordCommentHTMLTests.swift:210`) feeds aside-free input and asserts no `docmod-comments-rail` container. Backed by `transformDocmodComments` returning input unchanged when `rangeOfCommentsAside` finds nothing (`WordCommentHTML.swift:155-157`) and when the aside holds no parseable comments (`:161-163`).
- Criterion 7 — pass: `test_buildDocxHTML_preservesRenderAsyncAndCJKFonts` (`WordCommentHTMLTests.swift:60`) asserts `buildDocxHTML`'s output contains `docx.renderAsync` and `font-family: 'SimSun'`. Both present at `WordCommentHTML.swift:100` and `:31`.
