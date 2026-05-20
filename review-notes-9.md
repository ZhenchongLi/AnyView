### Claude

## Verdict
changes-requested

## Real issues

1. The feature does not exist. The diff adds a `.testTarget` and one marker test (`XCTAssertEqual(1, 1)`). That is the whole change. No `WordCommentHTML.swift`. No `buildDocxHTML`. No `transformDocmodComments`. No `docmodReadArguments`. `WebRenderer.swift` and `DocmodCLI.swift` are untouched. Criteria 2 through 7 ship zero code.

2. The named tests from the design doc are missing. `test_buildDocxHTML_containsCommentSidebarScaffold`, `test_transformDocmodComments_placesCommentInSidebar`, `test_docmodReadArguments_notRender`, `test_buildDocxHTML_noSidebarContainerWhenCommentFree`, `test_transformDocmodComments_noSidebarWhenNoAside`, `test_buildDocxHTML_preservesRenderAsyncAndCJKFonts` — none of them are in `Tests/AnyViewAppTests/WordCommentHTMLTests.swift`. The file holds one assertion that proves nothing about the feature.

3. The docx path still hands a base64 blob to `WebRenderer` with no sidebar CSS or JS hook. Word comments still do not show on the `.docx` path. The user-visible bug the issue opened against is unchanged.

4. The docmod and doct paths still call `DocmodCLI.render`, which runs `docmod render`. The design says that command strips the comment text and the `<aside>` block. Comments still do not show on the `.docmod` and `.doct` paths either.

## Questions

- The design notes that `docmod read` returns a JSON envelope, not raw HTML, and the HTML lives in the `html` field. When the read path lands, where does the JSON parse and `html` extraction live, and what happens on a malformed envelope — does it fall back to render, or surface an error to the user?
- The docx zero-comment test asserts on static markup only; the rail is built by JS at runtime and `swift test` cannot run it. The "no empty rail" guarantee for docx is unverified by the suite. How is that checked before merge — a manual run against a comment-free docx?

## Nits

None.

## Functional evidence
- Criterion 1 — pass: `swift test` from the package root builds `AnyViewAppTests` and runs `test_testTargetRuns`. Output: "Executed 1 test, with 0 failures"; `echo $?` after the run prints `EXIT=0`. `.testTarget(name: "AnyViewAppTests", dependencies: ["AnyViewApp"], path: "Tests/AnyViewAppTests")` added to `Package.swift`.
- Criterion 2 — fail: No `buildDocxHTML(base64:jszipScript:docxPreviewScript:)` exists. Grep across `Sources/` for `buildDocxHTML` returns nothing. The docx HTML is still built inline inside a `WebRenderer` instance method. No standalone function, no test calling it.
- Criterion 3 — fail: No `.docx-comments-rail` CSS and no JS hook to move `docx-preview` comment nodes anywhere. Grep for `docx-comments-rail` across `Sources/` returns nothing. No test asserts either substring.
- Criterion 4 — fail: No `transformDocmodComments(html:)` exists. Grep returns nothing. No fixed-input test placing a `cm1` comment from author `AI` into a sidebar.
- Criterion 5 — fail: No `docmodReadArguments(path:)` exists. The docmod/doct paths still call `DocmodCLI.render` (`docmod render`). No test asserts the argument list is not `["render", <path>]`.
- Criterion 6 — fail: Neither zero-comment test exists. No `test_buildDocxHTML_noSidebarContainerWhenCommentFree`, no `test_transformDocmodComments_noSidebarWhenNoAside`. The functions they would call do not exist.
- Criterion 7 — fail: No test asserts `docx.renderAsync` and the CJK `@font-face` block survive in the returned string, because the function that returns that string does not exist. The substrings still live inline in `WebRenderer.swift`, but the extraction-with-tests this criterion requires has not happened.
