### Claude

## Verdict
changes-requested

## Real issues

1. Criterion 6 has no test on either path. The criterion demands two unit tests: one asserting the docx HTML for a comment-free input carries no sidebar container, one asserting the docmod transform output for an aside-free input carries no sidebar container. Neither `test_buildDocxHTML_noSidebarContainerWhenCommentFree` nor `test_transformDocmodComments_noSidebarWhenNoAside` exists in `Tests/AnyViewAppTests/WordCommentHTMLTests.swift`. The behavior is in the code — `buildDocxHTML` builds the rail only in JS, `transformDocmodComments` returns input unchanged with no `<aside>` — but the criterion requires the assertions, and a future edit that pre-renders an empty rail would not be caught.

2. Criterion 7 has no test. The criterion demands one unit test asserting both `docx.renderAsync` and the CJK `@font-face` block remain in the returned docx HTML. No `test_buildDocxHTML_preservesRenderAsyncAndCJKFonts` exists. The only `SimSun` reference in the test file (`Tests/AnyViewAppTests/WordCommentHTMLTests.swift:54`) is a negative assertion that `loadDocxContent` dropped its inline copy — the opposite check. Both substrings live in `buildDocxHTML` (`Sources/AnyViewApp/WordCommentHTML.swift:100` and `:29-48`), but nothing pins them. Drop SimSun or break renderAsync in a later edit and `swift test` stays green.

## Questions

- Criterion 4 now passes: `transformDocmodComments` exists (`Sources/AnyViewApp/WordCommentHTML.swift:153`) and `test_transformDocmodComments_placesCommentInSidebar` asserts the sidebar container, author, text, and cm1 association. The docmod sidebar that the previous round flagged as missing is wired into both `loadDocmodContent` and `loadDoctContent`.
- The docmod rail emits `body { margin-right: 280px; }` inside a `<style>` nested in the `<aside>` (`WordCommentHTML.swift:187`). On the doct path the `<aside>` rail gets sliced out of `<body>`/`</body>` and embedded into `buildDoctHtml`'s preview region. The `position: fixed` rail pins to the viewport and the `margin-right` shifts the whole doct page body. Confirm against a real `.doct` file that the rail does not collide with the doct metadata layout — this is a browser-render property `swift test` can't see.
- The `<aside>`-survives-the-slice claim still rests on the design doc, not a fixture from real docmod 2.15.23 read output. The transform test feeds a hand-written input string. Confirm the real read envelope's `html` field places `<aside data-type="comments">` inside `<body>` so the doct slice keeps it.

## Nits

None.

## Functional evidence
- Criterion 1 — pass: `swift test` builds `AnyViewAppTests` and runs 7 tests, "Executed 7 tests, with 0 failures (0 unexpected)", exit zero. `.testTarget(name: "AnyViewAppTests", dependencies: ["AnyViewApp"], path: "Tests/AnyViewAppTests")` at `Package.swift:17-21`.
- Criterion 2 — pass: `buildDocxHTML(base64:jszipScript:docxPreviewScript:) -> String` at `Sources/AnyViewApp/WordCommentHTML.swift:9`, no `WebRenderer`/`NSView`/`WKWebView`; `test_buildDocxHTML_isCallableStandalone` calls it with stub scripts and asserts non-empty. Wired into production at `WebRenderer.swift:1480`.
- Criterion 3 — pass: `.docx-comments-rail` CSS at `WordCommentHTML.swift:51,57`, `moveCommentNodesToRail` JS hook at `:70-83` (called at `:101`, `:120`); `test_buildDocxHTML_containsCommentSidebarScaffold` asserts both `.docx-comments-rail` and `moveCommentNodesToRail` present.
- Criterion 4 — pass: `transformDocmodComments(html:)` at `WordCommentHTML.swift:153` parses the `<aside data-type="comments">` block and rebuilds each comment as a `docmod-comment-card` inside a `docmod-comments-rail`; `test_transformDocmodComments_placesCommentInSidebar` (`WordCommentHTMLTests.swift:66`) feeds a fixed input with `data-id="cm1"`, author `AI`, text "Consider rewording this phrase.", and `<mark data-id="cm1">`, asserting the rail, author, text, and cm1 association all present.
- Criterion 5 — pass: `DocmodCLI.docmodReadArguments(path:)` returns `["read", path]` at `DocmodCLI.swift:70`; `test_docmodReadArguments_notRender` asserts not `["render", path]` and first element `"read"`; `test_loadDocmodAndDoctContent_useReadNotRender` confirms `loadDocmodContent` (`WebRenderer.swift:1629`) and `loadDoctContent` (`WebRenderer.swift:1660`) both call `DocmodCLI.readHTML`, not `DocmodCLI.render`.
- Criterion 6 — fail: neither required test exists. No `test_buildDocxHTML_noSidebarContainerWhenCommentFree`, no `test_transformDocmodComments_noSidebarWhenNoAside` in `WordCommentHTMLTests.swift` (7 tests total, none for the empty-rail guarantee). The criterion requires two assertions; the diff adds zero.
- Criterion 7 — fail: no test asserts `docx.renderAsync` and the CJK `@font-face` block remain in the returned HTML. The lone `SimSun` reference at `WordCommentHTMLTests.swift:54` is a negative assertion (inline copy removed), not the required `test_buildDocxHTML_preservesRenderAsyncAndCJKFonts`.
