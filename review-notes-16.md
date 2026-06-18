### Claude

## Verdict
approve

## Real issues
None.

## Questions
- 数学公式回退分支（`catch`）渲染成功后只调 `moveCommentNodesToRail`，没调 `renderCommentsFromZip`。所以一份既带数学公式又带批注的 docx，触发回退后批注栏会空。验收 1 只要求正常路径读 comments.xml，回退路径不在范围内，这条不拦。但回退路径里那一次 `JSZip.loadAsync` 已经解过包，顺手补一句 `renderCommentsFromZip` 几乎零成本——以后想做可以考虑。
- `renderComments: true` 这个 docx-preview 选项还留在 opts 里（149 行）。设计里说它在 macOS 13 上本来就是死路，留着不影响验收。代码注释没说明「为什么开了 renderComments 还要自己解析」，读代码的人会困惑。不是硬性问题。

## Nits
- 第 49-50 行的 CSS 注释还写着「only present once the JS hook finds docx-preview comment nodes and moves them in」，只描述了 `moveCommentNodesToRail` 那条旧路径。现在 rail 也由 `renderCommentsFromZip` 建，注释没跟上。

## Functional evidence
- Criterion 1 — pass: 正常路径 `try` 块里 `await docx.renderAsync(...)` 后接 `await renderCommentsFromZip(bytes)`（WordCommentHTML.swift:152-154），`renderCommentsFromZip` 读 `zip.file('word/comments.xml')`（:95）。`test_buildDocxHTML_readsCommentsXmlInNormalPath` 断言 `word/comments.xml` 子串位置在数学公式回退标记之前，passed。
- Criterion 2 — pass: `getElementsByTagName('w:comment')` 遍历（:99,105），每条拼 `w:t` runs 的 `textContent` 成正文（:107-111），卡片 `rail.appendChild(card)`（:133）。`test_buildDocxHTML_appendsCardPerComment` 断言解析区含 `w:comment`/`w:t`/`docx-comments-rail`+`appendChild`，passed。
- Criterion 3 — pass: `comment.getAttribute('w:author')` 和 `getAttribute('w:date')`（:120-121），有任一则建 `.docx-comment-meta` 显示 `[author, date].filter(Boolean).join(' · ')`（:123-127）。`test_buildDocxHTML_cardShowsAuthorAndDate` 断言解析区含 `w:author`/`w:date`，passed。
- Criterion 4 — pass: `if (!commentsFile) return;`（:96）处理文件缺失，`if (comments.length === 0) return;`（:103）处理零批注，两条都在建 rail 之前。`test_buildDocxHTML_noRailWhenNoComments` 断言含 `comments.length === 0` 早退，配合既有 `test_buildDocxHTML_noSidebarContainerWhenCommentFree` 钉死无静态 rail，passed。
- Criterion 5 — pass: opts 仍含 `renderComments: true, renderChanges: true`（:149）。`test_buildDocxHTML_preservesRenderChanges` 断言 HTML 含 `renderChanges: true`，passed。
- 全套 `swift test --filter WordCommentHTMLTests`：17 tests, 0 failures。
