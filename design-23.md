# [cc] 语法高亮支持 Erlang

## Current state

`.erl` 文件走 `WebRenderer.loadCodeFile`。这个函数读源码、HTML 转义，再查 `langMap`
拿到语言名。`langMap["erl"]` 是 `"erlang"`，所以生成的 HTML 里有
`<code class="language-erlang">`。

页面里注入的只有 `highlightScript`，也就是打包进来的 `highlight.min.js` 这个定制构建
（`Sources/AnyViewApp/Resources/highlight.min.js`）。这个构建里没有 Erlang 语法。
highlight.js 在 `hljs.highlightAll()` 时找不到 `erlang` 这门语言，于是 `.erl` 文件
当纯文本显示，没有着色。

仓库里 LaTeX 已经踩过同样的坑，解法是单独打一个语法文件。
`Sources/AnyViewApp/Resources/hljs-latex.js` 是一段自执行脚本，结尾调用
`hljs.registerLanguage("latex", latex)` 把语法挂到全局 `hljs` 上。
`WebRenderer` 用 `hljsLatexScript`（`Bundle.module` 读出文件内容）持有它，
在 `loadTexFile` 里拼成 `latexGrammar = "<script>...</script>"` 注入 HTML，
位置在 `highlightInline` 之后。这样核心包先加载、注册了 `hljs`，语法文件再补挂上去。

要注意 `loadTexFile` 才注入 `latexGrammar`，`loadCodeFile` 没有。`.erl` 走的是
`loadCodeFile`，所以光加一个资源文件不够，还得在 `loadCodeFile` 里把它注进去。

## Approach

照搬 LaTeX 的做法，给 Erlang 单独加一个语法文件，不去重新构建 `highlight.min.js`。
重建整包要动 vendored 文件，以后升级 highlight.js 更麻烦。

三步：

1. 加资源文件 `Sources/AnyViewApp/Resources/hljs-erlang.js`。内容是 highlight.js 官方
   的 Erlang 语法，包成跟 `hljs-latex.js` 一样的自执行壳子，结尾
   `if(window.hljs){hljs.registerLanguage("erlang",erlang);}`。`Package.swift` 已经
   `.process("Resources")` 整个目录，新文件自动进资源包，不用改 `Package.swift`。

2. `WebRenderer` 加一个 `hljsErlangScript` 静态属性，跟 `hljsLatexScript` 一模一样，
   `Bundle.module.url(forResource: "hljs-erlang", withExtension: "js")` 读出来。

3. `loadCodeFile` 里，在 `highlightInline` 后面注入 Erlang 语法。最直接的做法是只在
   `lang == "erlang"` 时注入，避免给其它语言的页面塞用不上的脚本。注入位置必须在
   `highlightInline` 之后，跟 LaTeX 一样，保证 `hljs` 先存在再注册。

`erlang` 这个语言名跟 `langMap["erl"]` 的值对上，`registerLanguage` 注册的名字、
HTML 里 `language-erlang` 的 class、`langMap` 的映射三处一致，`hljs.highlightAll()`
才能匹配上。

本次只动 macOS 端。Linux 端的 `linux/resources/highlight.min.js` 同样缺 Erlang，
按 issue 留作后续，不在这次范围里。

## Acceptance criteria → tests

仓库没有 Swift 测试 target，`swift test` 跑不起来。下面两项验证按这个仓库能落地的
形式来设计：一项是构建后从资源包读文件，一项是对 `loadCodeFile` 生成的 HTML 做子串断言。

### 验收项 1 — Erlang 语法作为单独资源文件打进资源包，能从 Bundle.module 读出且非空

- Call chain: `WebRenderer.hljsErlangScript` 静态属性初始化 → `Bundle.module.url(forResource:withExtension:)` → `String(contentsOf:)`
- Test entry: `WebRenderer.hljsErlangScript`。读这个静态属性会触发资源包查找和文件读取，
  正好覆盖「打进了资源包」和「内容非空」两件事。
- Test: `verify-erlang-resource`，一段读 `WebRenderer.hljsErlangScript` 并断言非空的检查。
  放在 `Tests/manual/erlang-highlight.md` 里写清楚怎么跑：`swift build` 之后，
  确认 `.build/.../AnyView_AnyViewApp.bundle` 里有 `hljs-erlang.js`，文件非空；
  或在 `loadCodeFile` 加临时 `assert(!Self.hljsErlangScript.isEmpty)` 跑一次真实
  `.erl` 文件确认不触发。文件路径
  `Sources/AnyViewApp/Resources/hljs-erlang.js`。

### 验收项 2 — .erl 源码视图 HTML 里包含这段 Erlang 语法定义

- Call chain: `WebRenderer.load(filePath:)` → 扩展名命中 `codeExtensions` → `loadCodeFile(filePath:)` → 拼 HTML 字符串 → `webView.loadHTMLString`
- Test entry: `loadCodeFile`。传一个 `.erl` 路径进去，拿它拼出来的 HTML 字符串做断言。
  断言 HTML 里含 `hljsErlangScript` 的特征子串（比如
  `hljs.registerLanguage("erlang"`），且 `<code class="language-erlang">` 在场。
  这里不到 `loadHTMLString` 真正渲染那一步，因为验收项只要求 HTML「包含这段语法定义」，
  是对生成字符串的检查，不依赖 WKWebView 真渲染。
- Test: `verify-erlang-html-injection`。同样记在 `Tests/manual/erlang-highlight.md`：
  开一个真实 `.erl` 文件，在 `loadCodeFile` 生成 HTML 后打印或断言它含
  `registerLanguage("erlang"` 子串。文件路径 `Sources/AnyViewApp/WebRenderer.swift`
  的 `loadCodeFile`。

## Risks & trade-offs

- 没有自动化测试。这个仓库本来就没有 Swift 测试 target，两项验收只能靠手动跑真实
  `.erl` 文件加一次性断言确认。这是仓库现状决定的，不是这次新引入的问题。验证步骤写进
  `Tests/manual/erlang-highlight.md`，让后面的人能照着复现。
- 只在 `lang == "erlang"` 时注入语法文件，是省掉无关页面的额外脚本。代价是
  `loadCodeFile` 多一个针对 Erlang 的特判。LaTeX 当年走的是单独的 `loadTexFile`，
  没有这个特判；这次没法照搬，因为 `.erl` 和其它几十种代码扩展名共用 `loadCodeFile`。
  如果以后还有别的语言要补语法文件，这种一个个特判的写法会变啰嗦，到时候值得抽成一张
  「扩展名 → 语法文件」的表。这次只补 Erlang，先不抽。
- hljs-erlang.js 的语法内容直接取 highlight.js 官方的 Erlang 定义。要确认它跟仓库里
  `highlight.min.js` 的版本对得上，太新的语法文件可能用到旧核心包没有的辅助函数。
  挑版本时对照 `highlight.min.js` 的构建版本号。
