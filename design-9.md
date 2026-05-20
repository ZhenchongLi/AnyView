# [cc] AnyView: show Word comments in a right-side sidebar for docx and docmod/doct

## Current state

AnyView renders Word documents in a `WKWebView` through two paths inside `WebRenderer`.

The `.docx` path is `loadDocxContent` (`Sources/AnyViewApp/WebRenderer.swift:1473`). It reads the file, base64-encodes it, and builds one HTML string inline. That string loads the bundled `docx-preview` library and calls `docx.renderAsync` with `renderComments: true` (`WebRenderer.swift:1540`). No comments show up. `docx-preview` parses the comment data from the docx blob but does not lay out a sidebar, and the inline `<style>` block (`WebRenderer.swift:1486`) has no rules for comment elements. The Swift side never sees the comment text on this path. It only hands over the base64 blob.

The `.docmod` and `.doct` paths are `loadDocmodContent` (`WebRenderer.swift:1715`) and `loadDoctContent` (`WebRenderer.swift:1733`). Both call `DocmodCLI.render` (`Sources/AnyViewApp/DocmodCLI.swift:68`), which runs `docmod render <file>`. I ran docmod 2.15.23 on a document carrying one comment to see what each command emits.

`docmod render` keeps the highlight `<mark>` but strips the rest:

```html
<p class="Normal" ...>
  This is a paragraph with some
  <mark>annotated</mark>
  text in it.
</p>
```

There is no `data-id` on the `<mark>` and no `<aside>` block, so the comment author and text are gone. `docmod read` keeps both:

```html
<p data-id="6C9CC7BC" data-pstyle="Normal">
  This is a paragraph with some
  <mark data-id="cm1">annotated</mark>
  text in it.
</p>
...
<aside data-type="comments">
  <p data-id="cm1" data-author="AI" data-date="...">Consider rewording this phrase.</p>
</aside>
```

One catch: `docmod read` does not print raw HTML. It prints a JSON envelope and the HTML lives in the `html` field. The top-level keys are `status`, `command`, `input`, `html`, `summary`. So switching the docmod path to `read` means parsing JSON and pulling out `html`, not just swapping the argument.

The package has no test target. `Package.swift` declares one executable target and nothing else. The HTML for both paths is built inline inside methods that write straight to the web view and return nothing, so a test cannot reach the generated string today.

## Approach

### A testable seam, and where the test target lives

Add a `.testTarget` named `AnyViewAppTests` to `Package.swift`, with `dependencies: ["AnyViewApp"]` and `path: "Tests/AnyViewAppTests"`. That is enough for `swift test` to build and run. The functions under test must be reachable from the test target, so they need `public` (or `internal` plus `@testable import AnyViewApp`). I'll use `@testable import AnyViewApp` so nothing in the public surface changes.

The HTML-building logic moves out of the `WebRenderer` instance methods into free functions or an enum of static functions in their own file, `Sources/AnyViewApp/WordCommentHTML.swift`. None of these functions touch `NSView` or `WKWebView`. The instance methods keep their job of reading files and calling `loadHTMLString`, but they delegate string construction to the new functions.

### Docx path

Extract a function `buildDocxHTML(base64:jszipScript:docxPreviewScript:) -> String`. It takes the base64 string and the two library script strings as parameters. Passing the scripts in (instead of reading `Bundle.module` inside the function) keeps the function free of bundle lookups, so a test can call it with empty or stub script strings and still assert on the CSS, the JavaScript hook, the `renderAsync` call, and the font-face block. `loadDocxContent` becomes: read file, base64-encode, call `buildDocxHTML` passing `Self.jszipScript` and `Self.docxPreviewScript`, hand the result to the web view.

The function keeps everything that's there now and adds a comment sidebar scaffold:

- CSS for a right-side rail. A layout selector such as `.docx-comments-rail` plus rules that pin it to the right.
- JavaScript that runs after `docx.renderAsync` resolves, finds the comment nodes `docx-preview` produced, and moves them into the rail. If there are no comment nodes, it creates no rail and no container.

The "zero comments means no sidebar" criterion is a runtime property of that JavaScript, but the docx path can't be inspected from Swift because the comment data is inside the base64 blob. The test for the docx zero-comment case asserts on the *input the function received*: when called with a base64 string that the test treats as comment-free, the returned HTML contains no pre-rendered sidebar container element. The scaffold's rail is built by JavaScript only when comment nodes exist, so a static comment-free build carries the CSS and the hook but no container markup. The assertion targets the container element string, not the CSS class.

### Docmod and doct path

Add `docmodReadArguments(path:) -> [String]` returning `["read", path]`. `DocmodCLI` gets a sibling to `render` that runs those arguments, parses the JSON envelope, and returns the `html` field. `loadDocmodContent` and `loadDoctContent` call that instead of `DocmodCLI.render`. `loadDoctContent` currently slices out the `<body>...</body>` region of the render output for its preview; the read output also has a `<body>`, so that slice still works, and the `<aside>` block sits inside `<body>` after `</article>`, so it survives the slice.

Add `transformDocmodComments(html:) -> String`. It takes HTML that may contain an `<aside data-type="comments">` block plus inline `<mark data-id="cmN">` anchors and returns HTML where each comment sits in a right-side sidebar container, each card carrying the author and text and tied to its anchor id. When the input has no `<aside data-type="comments">` block, it returns HTML with no sidebar container. The docmod path runs this on the HTML it gets from `docmod read` before loading it.

I'll keep the transform string-based rather than pulling in an XML parser. The docmod read HTML is generated by docmod and has a predictable shape (`<aside data-type="comments">` wrapping `<p data-id=... data-author=...>` entries, `<mark data-id="cmN">` inline). A small, targeted transform is enough for read-only display and avoids a new dependency. The risk of that choice is in the trade-offs section.

### What stays out

No card vertical positioning, no leader lines, no light/dark theming assertions. Those render in the browser and `swift test` can't see them. No editing, replying, resolving, threaded replies, or done-state. No change to the docmod CLI itself; the path switches from `render` to `read`, which already emits comments.

## Acceptance criteria → tests

All tests live in `Tests/AnyViewAppTests/WordCommentHTMLTests.swift` unless noted, and use `@testable import AnyViewApp`.

1. `swift test` builds a test target and runs at least one test, exiting zero (requires the `.testTarget`) → satisfied by the test target existing and every test below running. The marker test is `test_testTargetRuns` in `WordCommentHTMLTests.swift`, a trivial assertion that proves the target builds and executes.

2. The docx HTML is produced by a function taking the bytes or base64 and returning a `String`, callable without `WebRenderer`, `NSView`, or `WKWebView` → `test_buildDocxHTML_isCallableStandalone` in `WordCommentHTMLTests.swift`. It calls `buildDocxHTML(base64:jszipScript:docxPreviewScript:)` with a base64 string and stub scripts, and asserts a non-empty `String` comes back.

3. The docx HTML contains the comment sidebar scaffold: CSS for a right rail and the JavaScript hook that moves `docx-preview` comment nodes into it → `test_buildDocxHTML_containsCommentSidebarScaffold` in `WordCommentHTMLTests.swift`. Asserts the CSS selector substring (`.docx-comments-rail`) and the JavaScript hook substring are both present in the returned string.

4. A function turns docmod HTML with an `<aside data-type="comments">` block into HTML placing each comment in a right-side sidebar; given one comment with `data-id="cm1"`, author `AI`, a known text, and a body `<mark data-id="cm1">`, the output has a sidebar container holding that author and text tied to the `cm1` anchor → `test_transformDocmodComments_placesCommentInSidebar` in `WordCommentHTMLTests.swift`. Feeds a fixed input string modeled on the real `docmod read` output captured above and asserts the sidebar container, the author `AI`, the comment text, and the `cm1` association are all present.

5. The docmod/doct path fetches a comment-bearing document from a docmod command other than `render`; the argument list comes from a function and a test asserts it is not `["render", <path>]` → `test_docmodReadArguments_notRender` in `WordCommentHTMLTests.swift` (or `DocmodCLITests.swift`). Asserts `docmodReadArguments(path:)` does not equal `["render", path]` and that its first element is `"read"`.

6. Zero comments means no sidebar container and no empty rail, on both paths → two tests. `test_buildDocxHTML_noSidebarContainerWhenCommentFree` in `WordCommentHTMLTests.swift` asserts the returned docx HTML for a comment-free input has no pre-rendered sidebar container element. `test_transformDocmodComments_noSidebarWhenNoAside` in `WordCommentHTMLTests.swift` feeds HTML with no `<aside data-type="comments">` block and asserts the output has no sidebar container.

7. The comment-free docx rendering is preserved: the docx HTML still has the `docx.renderAsync` call and the CJK `@font-face` block mapping Windows fonts to macOS fonts → `test_buildDocxHTML_preservesRenderAsyncAndCJKFonts` in `WordCommentHTMLTests.swift`. Asserts both the `docx.renderAsync` substring and a font-face substring (for example `font-family: 'SimSun'`) remain in the returned string.

## Risks & trade-offs

The docmod comment transform is string-based, not a real HTML parse. If a future docmod version changes the `<aside>` or `<mark>` shape, the transform silently produces a document with no sidebar instead of failing loudly. The test pins the transform against a fixed input modeled on docmod 2.15.23 output, so a shape change in docmod would not be caught until someone opens a real file. A real parser would be sturdier but pulls in a dependency this read-only feature doesn't otherwise need.

The issue states `docmod render` emits no `<mark>` anchors. In docmod 2.15.23 it actually emits `<mark>` without a `data-id` and without the `<aside>` block. The conclusion the issue draws still holds: `render` lacks the comment text and the anchor id, so the path must switch to `docmod read`. The added cost is that `read` returns a JSON envelope, so the docmod path now parses JSON to pull out the `html` field rather than using the output as-is. That parsing is a new failure point if docmod ever changes the envelope keys.

The docx zero-comment test asserts on static markup, not on what the browser actually renders. The rail is built by JavaScript at runtime from `docx-preview`'s comment nodes, and `swift test` can't run that JavaScript. So the test confirms the static build carries no sidewide container, but the real "no empty rail" guarantee for docx is checked by hand in the running app. That gap is inherent to the docx path keeping its comment data inside the base64 blob.
