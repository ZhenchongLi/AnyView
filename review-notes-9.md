### Claude

## Verdict
changes-requested

## Real issues

1. The docmod/doct path ships no sidebar. `loadDocmodContent` (`Sources/AnyViewApp/WebRenderer.swift:1629`) now fetches comment-bearing HTML through `DocmodCLI.readHTML`, then loads it into the web view untouched. The `<aside data-type="comments">` block reaches the browser exactly as docmod emits it — a plain block at the bottom of the document body, not a right-side sidebar. Open a `.docmod` file with one comment: the author and text dump under the text. Criterion 4 produces zero sidebar behavior.

2. `transformDocmodComments(html:)` does not exist. The design (`design-9.md:62`) names it; no source file defines it. Neither `loadDocmodContent` nor `loadDoctContent` runs any transform on the read HTML. There is no docmod sidebar CSS anywhere — `.docx-comments-rail` lives only in `buildDocxHTML`, and the docmod HTML never carries it. The "places each comment in a right-side sidebar" half of criterion 4 has no code path.

3. Criterion 4 has no test. The design names `test_transformDocmodComments_placesCommentInSidebar` against a fixed input with `data-id="cm1"`, author `AI`, and a `<mark data-id="cm1">` body run. No such test exists. The cm1-to-sidebar mapping is unverified.

4. Criterion 6 has no test on either path. `test_buildDocxHTML_noSidebarContainerWhenCommentFree` and `test_transformDocmodComments_noSidebarWhenNoAside` are both absent. The "zero comments means no empty rail" guarantee is unverified for docx and impossible for docmod (no transform to assert against). The criterion requires two tests; the diff adds neither.

5. Criterion 7 has no test. The design names `test_buildDocxHTML_preservesRenderAsyncAndCJKFonts` asserting both the `docx.renderAsync` substring and a CJK `@font-face` substring survive in the returned HTML. The only `SimSun` reference in the test file (`Tests/AnyViewAppTests/WordCommentHTMLTests.swift:54`) is a negative assertion that `loadDocxContent` dropped its inline copy — the opposite of criterion 7. The substrings exist in `buildDocxHTML` (`WordCommentHTML.swift:100` and `:29-48`); the required test pinning them does not. The next edit to that string can drop SimSun or break renderAsync and `swift test` stays green.

## Questions

- The docmod read output is sliced for the doct preview (`WebRenderer.swift:1659`) by `<body>`/`</body>`. Once a docmod transform exists, confirm the `<aside>` still survives that slice against real docmod 2.15.23 read output — design claims it sits inside `<body>` after `</article>`, but that needs a fixture, not a claim.
- The docx zero-comment guarantee is a runtime JS property; `swift test` cannot run the rail-building JS. The static-markup test (when written) confirms no pre-rendered container ships, but the real "no empty rail" check happens only in the browser. How is that verified before merge — a manual run against a comment-free docx?

## Nits

None.

## Functional evidence
- Criterion 1 — pass: `swift test` builds `AnyViewAppTests` and runs 6 tests, "Executed 6 tests, with 0 failures (0 unexpected)", exit zero. `.testTarget(name: "AnyViewAppTests", dependencies: ["AnyViewApp"], path: "Tests/AnyViewAppTests")` at `Package.swift:17-21`.
- Criterion 2 — pass: `buildDocxHTML(base64:jszipScript:docxPreviewScript:) -> String` at `Sources/AnyViewApp/WordCommentHTML.swift:9`, no `WebRenderer`/`NSView`/`WKWebView`; `test_buildDocxHTML_isCallableStandalone` calls it with stub scripts and asserts non-empty. Wired into production at `WebRenderer.swift:1480`.
- Criterion 3 — pass: `.docx-comments-rail` CSS at `WordCommentHTML.swift:51,57`, `moveCommentNodesToRail` JS hook at `:70-83` (called at `:101`, `:120`); `test_buildDocxHTML_containsCommentSidebarScaffold` asserts both substrings present.
- Criterion 4 — fail: no `transformDocmodComments(html:)` function (only design-9.md names it). The docmod/doct path loads raw `readHTML` output with no transform — the `<aside>` renders inline, not in a sidebar. No `test_transformDocmodComments_placesCommentInSidebar`.
- Criterion 5 — pass: `DocmodCLI.docmodReadArguments(path:)` returns `["read", path]` at `DocmodCLI.swift:70`; `test_docmodReadArguments_notRender` asserts not `["render", path]` and first element `"read"`; `test_loadDocmodAndDoctContent_useReadNotRender` confirms `loadDocmodContent` (`WebRenderer.swift:1629`) and `loadDoctContent` (`WebRenderer.swift:1658`) both call `DocmodCLI.readHTML`, not `DocmodCLI.render`.
- Criterion 6 — fail: neither zero-comment test exists. No `test_buildDocxHTML_noSidebarContainerWhenCommentFree`, no `test_transformDocmodComments_noSidebarWhenNoAside`. Test file holds 6 tests, none for the empty-rail guarantee.
- Criterion 7 — fail: no test asserts `docx.renderAsync` and the CJK `@font-face` block remain in the returned HTML. The lone `SimSun` reference at `WordCommentHTMLTests.swift:54` is a negative assertion (inline copy removed), not the required `test_buildDocxHTML_preservesRenderAsyncAndCJKFonts`.
