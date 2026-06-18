# [cc] docx 批注 UX:正文高亮 + 卡片与正文齐平对齐 + 双向定位

## Current state

`buildDocxHTML`（`Sources/AnyViewApp/WordCommentHTML.swift`）产出一整段 HTML，里面带一段浏览器跑的 JS。docx-preview 先把正文渲染进 `#container`，然后 `renderCommentsFromZip(bytes)` 用 JSZip 读 `word/comments.xml`，对每个 `w:comment` 拼一张卡片，append 进右侧的 `.docx-comments-rail`。

现在卡片只有正文文字、作者、日期三样东西，卡片之间纯靠 `margin-bottom: 12px` 自上而下堆。问题有三个：

- 卡片不带任何 id，没法和正文里被批注的那段话对上。
- 正文 DOM 完全没被碰过。`word/comments.xml` 只有批注本身的内容，被批注的区间记在 `word/document.xml` 里的 `commentRangeStart` / `commentRangeEnd` 标记，现在的代码不读这对标记，正文上看不出哪句话有批注。
- 卡片堆在右上角，竖直位置和正文无关。用户看着一句话，不知道哪张卡是它的。

docx-preview 自己渲染区间标记的方式在最低支持的 macOS 上不可见（CSS Custom Highlight API 要 Safari 17+），所以批注内容一直是我们自己从 zip 里解析的。区间标记这次也得自己从 DOM 里找。

## Approach

目标状态：正文里被批注的那段话有可见底色，右侧卡片竖直对齐到那段话的高度，点卡片滚到正文、点正文强调卡片。改动全在 `buildDocxHTML` 产出的那段 JS 和 CSS 里，不动 Swift 侧的函数边界。

按 id 配对是整条链的地基。`w:comment` 的 `w:id` 既写到卡片的 `data-comment-id`，也写到正文高亮 span 的 `data-comment-id`，两边用同一个值配对。

正文高亮怎么来。docx-preview 把 `commentRangeStart` / `commentRangeEnd` 渲染成 DOM 注释节点（`<!-- ... -->`），节点文本里带 `commentRangeStart` / `commentRangeEnd` 和对应的批注 id。新加一段 JS 在 `docx.renderAsync` 之后遍历 `#container` 的 DOM，用 `NodeFilter.SHOW_COMMENT` 的 `TreeWalker` 找到这对标记节点，把它们之间的正文节点包进一个 `<span data-comment-id="...">`。这段 span 加上前面那条 CSS 底色规则就有了可见高亮。

对齐怎么做。高亮 span 包好之后，它在页面里有了真实的 `offsetTop`。新加一段 JS 拿每张卡片配对 span 的 `offsetTop`，给卡片设 `top` 或 `transform: translateY(...)`，让卡片竖直位置跟着正文走。

双向定位。卡片绑 click，点的时候按 `data-comment-id` 找到正文里同 id 的高亮 span，`scrollIntoView` 滚过去。高亮 span 也绑 click，点的时候找到同 id 的卡片，滚到它并加一个强调样式（具体样式留给实现，验收只认「滚到卡 + 加强调 class」这个行为）。

避让。多张卡片的 `offsetTop` 可能挨得很近甚至重叠。定位时按 `offsetTop` 排序，从上往下扫，记住上一张卡片的底边，如果当前卡片算出来的 `top` 比上一张底边还小，就把它往下推到不重叠为止。最朴素的往下堆，不做力导布局。

这段对齐 / 定位 / 避让的 JS 接在 `renderCommentsFromZip` 之后跑，因为它要先有卡片、也要先有高亮 span。

测试只能断言 `buildDocxHTML` 产出的 HTML / JS 字符串里**包含**这些逻辑标记。卡片真对齐到像素、点击真滚到位，要靠 WKWebView 运行时排版，无头测试测不到，得手动开带批注的 docx 验。下面每条验收对应一个字符串断言测试，沿用 `WordCommentHTMLTests.swift` 里已有的写法：先 `range(of: "word/comments.xml")` 和 `range(of: "数学公式")` 圈出正常渲染路径那段区间，再在区间里断言子串存在。

## Acceptance criteria → tests

### 验收 1 — 卡片带 data-comment-id
- Call chain: none(直接读 `buildDocxHTML` 返回的字符串)
- Test entry: `buildDocxHTML` 返回值。这条验收没有真实调用链能到运行时排版，无头测试只能断言产出字符串里卡片构建那段 JS 写了 `data-comment-id`，取值来自 `w:id`。
- Test: `test_buildDocxHTML_cardCarriesCommentId` in `Tests/AnyViewAppTests/WordCommentHTMLTests.swift`

### 验收 2 — 正文高亮的 CSS 规则
- Call chain: none(直接读 `buildDocxHTML` 返回的字符串)
- Test entry: `buildDocxHTML` 返回值。断言 `<style>` 里有一条匹配带 `data-comment-id` 的 span 的规则，且带一个可见底色（`background`）。
- Test: `test_buildDocxHTML_highlightSpanHasBackgroundCss` in `Tests/AnyViewAppTests/WordCommentHTMLTests.swift`

### 验收 3 — JS 把区间标记之间的正文包进高亮 span
- Call chain: none(直接读 `buildDocxHTML` 返回的字符串)
- Test entry: `buildDocxHTML` 返回值。在正常渲染路径区间里断言 JS 提到 `commentRangeStart` 和 `commentRangeEnd`，并构建一个带 `data-comment-id` 的 span 把这对标记之间的正文包进去。
- Test: `test_buildDocxHTML_wrapsCommentRangeInSpan` in `Tests/AnyViewAppTests/WordCommentHTMLTests.swift`

### 验收 4 — 用 offsetTop 给卡片做竖直定位
- Call chain: none(直接读 `buildDocxHTML` 返回的字符串)
- Test entry: `buildDocxHTML` 返回值。断言 JS 读了配对 span 的 `offsetTop`，并把它写进卡片的 `top` 或 `transform: translateY`。
- Test: `test_buildDocxHTML_positionsCardByOffsetTop` in `Tests/AnyViewAppTests/WordCommentHTMLTests.swift`

### 验收 5 — 点卡片滚到正文高亮
- Call chain: none(直接读 `buildDocxHTML` 返回的字符串)
- Test entry: `buildDocxHTML` 返回值。断言 JS 给卡片绑了 click 监听（`addEventListener` + `'click'`），handler 里按 `data-comment-id` 找正文 span 并滚过去（`scrollIntoView`）。
- Test: `test_buildDocxHTML_cardClickScrollsToHighlight` in `Tests/AnyViewAppTests/WordCommentHTMLTests.swift`

### 验收 6 — 点正文高亮强调卡片
- Call chain: none(直接读 `buildDocxHTML` 返回的字符串)
- Test entry: `buildDocxHTML` 返回值。断言 JS 给高亮 span 绑了 click 监听，handler 里按同 id 找卡片、滚到它并加一个强调 class。
- Test: `test_buildDocxHTML_highlightClickEmphasizesCard` in `Tests/AnyViewAppTests/WordCommentHTMLTests.swift`

### 验收 7 — 卡片高度相近时往下推避让
- Call chain: none(直接读 `buildDocxHTML` 返回的字符串)
- Test entry: `buildDocxHTML` 返回值。断言定位 JS 里有避让逻辑：拿上一张卡片的底边和当前卡片算出的 top 比，重叠就把后一张往下推。
- Test: `test_buildDocxHTML_stacksOverlappingCards` in `Tests/AnyViewAppTests/WordCommentHTMLTests.swift`

## Risks & trade-offs

- 测试只盖到字符串包含，盖不到行为。七条验收全是「产出 JS 里有没有这段逻辑」，断言通过不等于卡片真对齐、点击真滚到位。区间标记节点找错、`offsetTop` 在分页布局下读到 0、避让算错方向，这些无头测试一个都抓不到，只能手动开带批注的 docx 看。这是 issue 已经写明并接受的限制，不是这次能消掉的。

- 区间标记的 DOM 形态依赖 docx-preview。我们假设 `commentRangeStart` / `commentRangeEnd` 渲染成带 id 的注释节点。docx-preview 换实现（比如改成 marker 元素、或换 id 写法）会让找标记的 JS 失效。和现有 `renderCommentsFromZip` 自己解析 zip 一样，这是为了绕开最低 macOS 不支持的 Custom Highlight API 付出的代价。找不到标记时应当让正文照常显示、只是没有高亮，不要抛错把整页渲染搞挂。

- 字符串断言会把实现写法钉死一部分。断言里出现的子串（`offsetTop`、`scrollIntoView`、`commentRangeStart` 等）会变成实现必须照着写的词。换等价写法（比如不用 `scrollIntoView` 改用 `scrollTo`）会让测试变红。验收里挑的都是这件事绕不开的核心 API 名，不是随便选的措辞，把这个风险压到最低。

- 避让是最朴素的往下堆。卡片只往下推、不往上挪，正文底部批注密集时卡片会越堆越靠下、和正文脱节。issue 的 open question 已经说明只做朴素堆叠、不做力导布局，这是有意的取舍。
