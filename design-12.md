# [cc] Markdown 代码块含空行时被拆成多个灰框

## Current state

Markdown 预览走 `WebRenderer`。真正的 markdown → HTML 变换是塞在 HTML 模板里的浏览器端 JS 函数 `md(s)`，在 `Sources/AnyViewApp/WebRenderer.swift` 第 788–829 行。

`md()` 的处理顺序是这样的：

1. 第 790–798 行：先用正则把围栏代码块换掉。mermaid 块换成 `<div data-mermaid-placeholder="N"></div>` 这样的占位符；其它语言的块直接换成最终的 `<pre><code>...</code></pre>`，代码内容用 `esc()` 转义后原样塞进去，里面还带着换行。
2. 第 799–822 行：表格、标题、列表、行内样式等块级和行级正则，都是在整个字符串上跑。
3. 第 823 行：段落正则 `/^(?!<[hupoltbd]|<li|<bl|<hr|<im|<a )(.+)$/gm`。它逐行扫整个字符串，把不像已知块级标签开头的非空行包成 `<p>`。
4. 第 825–827 行：最后才把 mermaid 占位符还原成 `<div class="mermaid">`。

问题出在第 1 步和第 3 步的配合上。普通代码块在第 1 步就变成了带内部换行的 `<pre><code>...</code></pre>` 文本，之后它就跟普通正文一样躺在字符串里。等第 3 步的段落正则逐行跑过来：

- `<pre><code>` 这一行因为以 `<p` 开头被负向前瞻挡掉，不包 `<p>`。
- 代码块内部的普通代码行（不以那些标签前缀开头）会被包上 `<p>`。
- 代码块内部如果有空行，`(.+)` 匹配不到空行，但空行把代码内容在视觉和 DOM 上断开，配合外面 CSS 的灰框样式，一个代码块就裂成两个灰框。

mermaid 块没这个毛病，因为它在第 1 步只留下一个单行的占位符 `<div>`，中间的代码被抽到 JS 数组里存着，等所有正则都跑完了第 4 步才还原。占位符是单行、以 `<` 开头，段落正则碰不到它。

## Approach

把普通围栏代码块也改成"先抽占位符、最后还原"，跟 mermaid 现在的做法对齐。

具体说，第 790–798 行那个 replace 回调里，非 mermaid 的分支不再直接返回 `<pre><code>...</code></pre>`，而是：

- 把这个块要生成的最终 HTML（带 `class` 和转义后的代码、保留内部换行）存进一个数组，比如 `codeBlocks`。
- 返回一个单行占位符，比如 `<div data-code-placeholder="N"></div>`，N 是数组下标。

然后在 `md()` 末尾、`return s` 之前，加一条还原正则，把 `<div data-code-placeholder="N"></div>` 换回 `codeBlocks[N]` 里存的 `<pre><code>...</code></pre>`。位置要放在第 823 行段落正则之后，这样段落正则跑的时候看到的只是单行占位符，不会去碰代码内容。

为什么用占位符而不是给段落正则加一个"跳过 `<pre>` 到 `</pre>` 之间"的规则：现在所有块级/行级正则都是在整串上无状态地跑 `gm`，要让它们成对地识别 `<pre>...</pre>` 的边界、并在里面停手，得给好几条正则都加跨行状态判断，容易顾此失彼。占位符方案是仓库里已经验证过的同一套路，改动集中在一处，风险小。

还原占位符时直接把存好的字符串塞回去就行，不需要再 `esc()`——内容在抽取时已经转义过了，这一点跟 mermaid 还原时再 `esc(mermaidBlocks[...])` 不一样，别照抄那行。

代码内容存进数组时不要 `.trim()` 掉换行结构里有意义的部分。现在第 797 行用的是 `esc(code.trim())`，`trim()` 去掉的是代码块首尾的空白，这个行为保持不变即可；要保住的是代码块"内部"的空行，而 `trim()` 本来就不动内部。

测试入口：把 `md()` 抽成一个可以在 node 下加载的纯函数来跑。`md()` 现在是 HTML 模板字符串里的一段 JS，没有 JS 测试框架。Dev 需要先把这段函数变成可被 node 引入的形式（例如抽到一个单独的 `.js` 资源文件，Swift 模板再引用它；或在测试里从源码字符串中截取这段函数 eval 出来）。怎么抽由 Dev 定，但抽出来后这四条验收都是对同一个纯函数 `md(inputString) -> outputString` 的断言。`esc()` 是 `md()` 的依赖，抽的时候要带上。

下面"测试入口"统一写成 `md()`，指的就是这个被抽出来、在 node 下可直接调用的纯函数。

## Acceptance criteria → tests

### 验收 1 — 含空行的 go 代码块只渲染出一个 `<pre><code>`
- Call chain: 测试构造一段含空行的 ```` ```go ```` markdown 字符串 → `md(s)` → 代码块抽占位符 → 各正则跑完 → 占位符还原成 `<pre><code>`
- Test entry: `md()`（被测纯函数的唯一入口，没有跳过任何层）
- Test: `test_go_block_with_blank_line_yields_single_pre_code` in `Tests/WebRendererMdTests/md.test.js`

### 验收 2 — 代码块内部空行在输出里保留
- Call chain: 同验收 1，断言点在 `md()` 返回的字符串上，检查 `<pre><code>` 和 `</code>` 之间仍含那一行空行（换行结构未被吞）
- Test entry: `md()`
- Test: `test_blank_line_inside_code_block_preserved` in `Tests/WebRendererMdTests/md.test.js`

### 验收 3 — 代码块内容里不含 `<p>` 等被段落/行级正则注入的标签
- Call chain: 同验收 1，断言点在 `md()` 返回字符串中 `<pre><code>...</code></pre>` 这一段子串上，检查其中不出现 `<p>`
- Test entry: `md()`
- Test: `test_code_block_content_has_no_injected_tags` in `Tests/WebRendererMdTests/md.test.js`

### 验收 4 — 标题、无序列表、表格、普通段落仍各自正确
- Call chain: 测试构造一段同时含 `#` 标题、`-` 列表、`|` 表格、普通段落的 markdown → `md(s)` → 各块级/行级正则 → 返回 HTML
- Test entry: `md()`
- Test: `test_mixed_document_keeps_h1_ul_table_p` in `Tests/WebRendererMdTests/md.test.js`

## Risks & trade-offs

- `md()` 现在嵌在 Swift 文件的 HTML 模板字符串里，没有现成的 JS 测试通道。要测它就得把这段函数抽出来在 node 下跑。抽取本身会动到 `WebRenderer.swift` 的模板组织方式（这部分在本 issue 范围内，因为验收点名要测 `md()`）。如果抽法选得不好，可能让模板和测试两份 `md()` 代码漂移。降低办法是只保留一份源，测试从那一份加载，而不是复制粘贴一份到测试里。
- 占位符用的是 `data-code-placeholder` 这类属性。如果用户的 markdown 正文里真的写了一模一样的字符串，理论上会被误还原。mermaid 现有方案也有同样的暴露面，这里不新增风险，沿用即可，不单独防护。
- 这次只改普通代码块的处理时机，不碰 mermaid、不碰 hljs 着色，符合 Out of scope。代码块内部如果本来就含像 `# foo` 这样的行，改之前会被错当成标题，改之后因为整块被占位符挡住，反而不会再被标题正则误伤——这是顺带的正确性提升，不属于新增范围。
