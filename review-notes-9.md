### Claude

## Verdict
changes-requested

## Real issues

1. Criterion 6 is half-covered. The criterion demands two tests: one for the docx path, one for the docmod path. The docx half landed this round — `test_buildDocxHTML_noSidebarContainerWhenCommentFree` (`Tests/AnyViewAppTests/WordCommentHTMLTests.swift:42`) asserts the comment-free docx build carries no `<div class="docx-comments-rail">` container. The docmod half is still missing. No test feeds `transformDocmodComments` an input with no `<aside data-type="comments">` block and asserts the output carries no `docmod-comments-rail` container. The behavior is in the code — `transformDocmodComments` returns the input unchanged when `rangeOfCommentsAside` finds nothing (`Sources/AnyViewApp/WordCommentHTML.swift:154-156`) — but the criterion requires the assertion, and a later edit that injects an empty rail would pass `swift test` untouched.

2. Criterion 7 has no test. The criterion demands one unit test asserting both `docx.renderAsync` and the CJK `@font-face` block remain in the returned docx HTML. No such test exists. The only `SimSun` reference in the test file (`Tests/AnyViewAppTests/WordCommentHTMLTests.swift:74`) is a negative assertion that `loadDocxContent` dropped its inline copy — the opposite check. Both substrings live in `buildDocxHTML` (`Sources/AnyViewApp/WordCommentHTML.swift:100` and `:29-48`), but nothing pins them in the function's output. Drop SimSun or break renderAsync in a later edit and `swift test` stays green.

## Questions

- The docx error path lost behavior in the move to `buildDocxHTML`. The old inline string showed the exception detail on render failure (`'渲染失败: ' + (e2 && e2.message ? e2.message : e2)`) and cleared the math-fallback status after 3 seconds (`setTimeout(... 3000)`). `buildDocxHTML` now shows a fixed `'无法渲染文档'` and leaves the `'数学公式已转为占位符'` status pinned (`Sources/AnyViewApp/WordCommentHTML.swift:121,124`). Intentional, or dropped in the extraction?
- The docmod rail emits `body { margin-right: 280px; }` inside a `<style>` nested in the `<aside>` (`WordCommentHTML.swift:187`). On the doct path the `<aside>` rail gets sliced out of `<body>`/`</body>` (`WebRenderer.swift:1662-1665`) and embedded into `buildDoctHtml`'s preview region. The `position: fixed` rail pins to the viewport and `margin-right` shifts the whole doct page. Confirm against a real `.doct` file that the rail does not collide with the doct metadata layout — `swift test` can't see this.
- The `<aside>`-survives-the-slice claim rests on the design doc, not a fixture from real docmod 2.15.23 read output. The transform test feeds a hand-written input string. Confirm the real read envelope's `html` field places `<aside data-type="comments">` inside `<body>` so the doct slice keeps it.

## Nits

None.

## Functional evidence
- Criterion 1 — pass: `swift test` builds `AnyViewAppTests` and runs 8 tests, "Executed 8 tests, with 0 failures (0 unexpected)", exit zero. `.testTarget(name: "AnyViewAppTests", dependencies: ["AnyViewApp"], path: "Tests/AnyViewAppTests")` at `Package.swift:17-21`.
- Criterion 2 — pass: `buildDocxHTML(base64:jszipScript:docxPreviewScript:) -> String` at `Sources/AnyViewApp/WordCommentHTML.swift:9`, no `WebRenderer`/`NSView`/`WKWebView`; `test_buildDocxHTML_isCallableStandalone` calls it with stub scripts and asserts non-empty. Wired into production at `WebRenderer.swift:1480`.
- Criterion 3 — pass: `.docx-comments-rail` CSS at `WordCommentHTML.swift:51,57`, `moveCommentNodesToRail` JS hook at `:70-83` (called at `:101`, `:120`); `test_buildDocxHTML_containsCommentSidebarScaffold` asserts both `.docx-comments-rail` and `moveCommentNodesToRail` present.
- Criterion 4 — pass: `transformDocmodComments(html:)` at `WordCommentHTML.swift:153` parses the `<aside data-type="comments">` block and rebuilds each comment as a `docmod-comment-card` inside a `docmod-comments-rail`; `test_transformDocmodComments_placesCommentInSidebar` (`WordCommentHTMLTests.swift:86`) feeds a fixed input with `data-id="cm1"`, author `AI`, text "Consider rewording this phrase.", and `<mark data-id="cm1">`, asserting the rail, author, text, and cm1 association all present.
- Criterion 5 — pass: `DocmodCLI.docmodReadArguments(path:)` returns `["read", path]` at `DocmodCLI.swift:70`; `test_docmodReadArguments_notRender` asserts not `["render", path]` and first element `"read"`; `test_loadDocmodAndDoctContent_useReadNotRender` confirms `loadDocmodContent` (`WebRenderer.swift:1629`) and `loadDoctContent` (`WebRenderer.swift:1660`) both call `DocmodCLI.readHTML`, not `DocmodCLI.render`.
- Criterion 6 — fail: only the docx half is tested. `test_buildDocxHTML_noSidebarContainerWhenCommentFree` (`WordCommentHTMLTests.swift:42`) covers the docx path. No test covers the docmod path — no `test_transformDocmodComments_noSidebarWhenNoAside` feeding an aside-free input and asserting no `docmod-comments-rail` container. The criterion requires two tests; one exists.
- Criterion 7 — fail: no test asserts `docx.renderAsync` and the CJK `@font-face` block remain in the returned HTML. The lone `SimSun` reference at `WordCommentHTMLTests.swift:74` is a negative assertion (inline copy removed), not the required check on `buildDocxHTML`'s output.
