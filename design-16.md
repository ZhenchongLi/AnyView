# [cc] docx 批注在默认渲染路径下不显示(右侧批注栏为空)

## Current state

带批注的 docx 用默认模式（WebKit + docx-preview）打开时，右侧批注栏不出现。同一文档的修订（`w:ins`/`w:del`）正常，只有批注看不到。

走的代码在 `Sources/AnyViewApp/WordCommentHTML.swift` 的 `buildDocxHTML`。它生成一整段 HTML，里面：

- 给 docx-preview 开了 `renderComments: true` 和 `renderChanges: true`（第 97 行）。
- 渲染成功后调 `moveCommentNodesToRail(container)`（第 101、120 行），想把 docx-preview 渲染出来的批注节点搬进 `.docx-comments-rail`。

问题出在 docx-preview 渲染批注的方式上。它不产出一块可见的批注内容节点，而是用 CSS Custom Highlight API 加一个 `display:none` 的悬浮气泡。所以 `moveCommentNodesToRail` 用 `querySelectorAll('.docx-comment, [class*="comment"], .docx-comments > *')` 去找，什么都搬不到。而且 Highlight API 要 Safari 17+，最低支持的 macOS 13 上 WKWebView 是 Safari 16，连高亮本身都出不来。

页面里已经加载了 JSZip。`buildDocxHTML` 在数学公式回退分支里用它解过 docx，读 `word/document.xml`（第 107-117 行）。但正常渲染路径里没碰 `word/comments.xml`。

`WebRenderer.loadDocxContent`（`WebRenderer.swift:1439`）把 base64、JSZip 脚本、docx-preview 脚本传给 `buildDocxHTML`，整段 HTML 直接 `loadHTMLString` 给 WKWebView。docx 字节只以 base64 形式存在页面里，Swift 侧不解压。

## Approach

不再依赖 docx-preview 渲染批注，也不再依赖 `moveCommentNodesToRail` 去搬它的节点。改成在 `buildDocxHTML` 的 JS 里自己解析批注。

正常渲染路径里，docx-preview 渲染完之后，用页面已加载的 JSZip 解 docx 包，读 `word/comments.xml`。对里面每个 `w:comment`，自己拼一张卡片追加到 `.docx-comments-rail`。卡片带批注正文文字；如果该 `w:comment` 有 `w:author` 和 `w:date`，卡片也显示作者和时间。

`.docx-comments-rail` 只在解析出至少一个 `w:comment` 时才创建。`word/comments.xml` 不存在、或里面没有任何 `w:comment` 时，不建卡片也不建批注栏。这跟现在 `moveCommentNodesToRail` 的「没节点就不建栏」行为一致。

解析放在浏览器 JS 里，不新写 Swift 侧解析函数。原因在 issue 里写明：docx 字节只以 base64 存在页面里，Swift 侧不解压。

`renderChanges: true` 保留不动。修订的渲染靠 docx-preview，本次改动不碰它，所以 `w:ins`/`w:del` 行为不变。

`renderComments` 这个选项可以保留也可以去掉——docx-preview 那条批注渲染本来就在 macOS 13 上出不来。本设计不强求改它，验收不打在这个选项上。

### 验收断言打在哪

测试沿用 #9 那批 docx 批注测试的做法：调 `buildDocxHTML(base64:jszipScript:docxPreviewScript:)`，对它返回的 HTML 字符串做子串断言。这是生产路径——`WebRenderer.loadDocxContent` 委托给 `buildDocxHTML`，已有回归测试 `test_loadDocxContent_delegatesToBuildDocxHTML` 钉死这个委托关系。

这里没有无头 WKWebView 的端到端测试能真正跑 JS 渲染批注。所以断言生成的 HTML 是否包含解析 `comments.xml`、拼卡片、读作者/时间这些逻辑的代码。测试看的是 HTML 里有没有这些字符串，不是运行结果。

## Acceptance criteria → tests

### 验收 1 — 正常路径读 comments.xml
- Call chain: none（直接读 `buildDocxHTML` 的返回字符串）。`loadDocxContent` 把 HTML 写进 WKWebView，单测看不到运行结果，唯一能观测的就是函数产出的 HTML 文本。
- Test entry: `buildDocxHTML` 的返回值。
- Test: `test_buildDocxHTML_readsCommentsXmlInNormalPath` in `Tests/AnyViewAppTests/WordCommentHTMLTests.swift`
- 断言：HTML 含 `word/comments.xml`。为了把「正常路径读」和「数学公式回退分支读 document.xml」分开，断言 `word/comments.xml` 出现的位置在 `docx.renderAsync` 第一次成功调用相关的代码段里，不只出现在 catch 回退块里。具体做法：断言含 `word/comments.xml` 这个子串（现有 HTML 里没有），且该子串不止一次或不在 `数学公式` 回退文案附近——Dev 写测试时按生成 HTML 的实际结构挑一个稳定锚点。

### 验收 2 — 每个 w:comment 追加一张带正文的卡片
- Call chain: none（直接读 `buildDocxHTML` 的返回字符串）。
- Test entry: `buildDocxHTML` 的返回值。
- Test: `test_buildDocxHTML_appendsCardPerComment` in `Tests/AnyViewAppTests/WordCommentHTMLTests.swift`
- 断言：HTML 里的 JS 含遍历 `w:comment` 的逻辑（子串 `w:comment`），且含把卡片追加进批注栏的逻辑（子串 `docx-comments-rail` 配合 `appendChild` 或建卡片的代码），并含取批注正文文字的部分（如 `w:t`）。

### 验收 3 — 卡片显示作者和时间
- Call chain: none（直接读 `buildDocxHTML` 的返回字符串）。
- Test entry: `buildDocxHTML` 的返回值。
- Test: `test_buildDocxHTML_cardShowsAuthorAndDate` in `Tests/AnyViewAppTests/WordCommentHTMLTests.swift`
- 断言：HTML 里的 JS 读 `w:comment` 的 `w:author` 和 `w:date` 属性（子串 `w:author` 和 `w:date`），并把它们放进卡片。

### 验收 4 — 无批注时不建卡片也不建批注栏
- Call chain: none（直接读 `buildDocxHTML` 的返回字符串）。无头测试跑不了 JS，断言落在「代码里有这个判空守卫」上，不是落在「跑出来真没建栏」上。
- Test entry: `buildDocxHTML` 的返回值。
- Test: `test_buildDocxHTML_noRailWhenNoComments` in `Tests/AnyViewAppTests/WordCommentHTMLTests.swift`
- 断言：批注解析这段 JS 在没拿到 `word/comments.xml`、或解析出 0 个 `w:comment` 时提前 return，不创建 `.docx-comments-rail` 容器。断言 HTML 仍不含静态的 `<div class="docx-comments-rail">` 元素（现有 `test_buildDocxHTML_noSidebarContainerWhenCommentFree` 已钉这一条，本测试钉解析分支里的判空守卫，比如含 `comments.length === 0` 之类的早退条件）。

### 验收 5 — renderChanges 保留，修订行为不变
- Call chain: none（直接读 `buildDocxHTML` 的返回字符串）。
- Test entry: `buildDocxHTML` 的返回值。
- Test: `test_buildDocxHTML_preservesRenderChanges` in `Tests/AnyViewAppTests/WordCommentHTMLTests.swift`
- 断言：HTML 含 `renderChanges: true`。这是回归钉，防止本次改批注时把修订选项一起动了。

## Risks & trade-offs

- 测试只断言生成的 HTML 文本里有没有相应的代码字符串，断不了批注真渲染出来。一段语法正确但逻辑写错的 JS 照样能让所有断言过。这是这条路（base64 进页面、无无头 WKWebView 端到端测试）固有的限制，#9 那批测试也是这么做的。批注真不真能显示，得靠手动开一份带批注的 docx 验。
- 自己用 JSZip 在浏览器里解 `comments.xml` 拼卡片，等于把 docx 批注的 XML 结构知识从 docx-preview 挪到我们自己的 JS 里。以后 Word 批注 XML 有边角情况（嵌套批注、批注引用 `commentsExtended.xml` 里的回复线程等），得我们自己处理。本设计只保证基本的批注正文、作者、时间可见，回复线程之类不在范围内。
- 批注卡片和正文之间的连线/高亮锚点不做（issue 里列为 out of scope）。批注栏里能看到内容，但看不出某条批注对应正文哪一段。
- docx-preview 的 `renderComments` 那条死路本设计没强制清掉。留着它不影响验收，但读代码的人可能困惑「为什么开了 renderComments 还要自己解析」。Dev 实现时可以顺手在注释里说明，或去掉该选项——不属于硬性验收。
