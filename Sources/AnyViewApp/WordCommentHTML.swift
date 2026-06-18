import Foundation

/// Builds the HTML string the `.docx` path hands to the web view.
///
/// The base64-encoded document bytes and the two library scripts are passed in
/// as parameters so this function performs no bundle lookups and can be called
/// from a unit test with stub scripts, without constructing `WebRenderer`, any
/// `NSView`, or a `WKWebView`.
func buildDocxHTML(base64: String, jszipScript: String, docxPreviewScript: String) -> String {
    return """
    <!DOCTYPE html>
    <html>
    <head>
    <meta charset="UTF-8">
    <meta name="color-scheme" content="light">
    <style>
        * { box-sizing: border-box; }
        html, body { margin: 0; padding: 0; background: #e5e7eb; }
        #container { padding: 24px 0; min-height: 100vh; }
        .docx-wrapper { background: transparent !important; padding: 0 !important; }
        .docx-wrapper > section.docx { box-shadow: 0 2px 8px rgba(0,0,0,0.12); margin: 0 auto 16px; }
        #status { position: fixed; top: 12px; right: 16px; z-index: 9999;
                  padding: 6px 12px; font: 12px -apple-system, sans-serif;
                  background: rgba(0,0,0,0.7); color: #fff; border-radius: 4px;
                  backdrop-filter: blur(8px); }
        #status:empty { display: none; }
        #status.error { background: #b91c1c; }
        /* Map Windows CJK fonts to macOS equivalents (normal + bold) */
        @font-face { font-family: '宋体'; font-weight: normal; src: local('STSong-Light'), local('STSong'); }
        @font-face { font-family: '宋体'; font-weight: bold; src: local('STSong'); }
        @font-face { font-family: 'SimSun'; font-weight: normal; src: local('STSong-Light'), local('STSong'); }
        @font-face { font-family: 'SimSun'; font-weight: bold; src: local('STSong'); }
        @font-face { font-family: '微软雅黑'; font-weight: normal; src: local('PingFang SC'), local('PingFang SC Regular'); }
        @font-face { font-family: '微软雅黑'; font-weight: bold; src: local('PingFang SC Semibold'), local('PingFang SC Medium'); }
        @font-face { font-family: 'Microsoft YaHei'; font-weight: normal; src: local('PingFang SC'), local('PingFang SC Regular'); }
        @font-face { font-family: 'Microsoft YaHei'; font-weight: bold; src: local('PingFang SC Semibold'), local('PingFang SC Medium'); }
        @font-face { font-family: '黑体'; font-weight: normal; src: local('STHeiti'); }
        @font-face { font-family: '黑体'; font-weight: bold; src: local('STHeiti Medium'); }
        @font-face { font-family: 'SimHei'; font-weight: normal; src: local('STHeiti'); }
        @font-face { font-family: 'SimHei'; font-weight: bold; src: local('STHeiti Medium'); }
        @font-face { font-family: '楷体'; src: local('STKaiti'); }
        @font-face { font-family: 'KaiTi'; src: local('STKaiti'); }
        @font-face { font-family: '仿宋'; src: local('STFangsong'); }
        @font-face { font-family: 'FangSong'; src: local('STFangsong'); }
        @font-face { font-family: '等线'; font-weight: normal; src: local('PingFang SC'); }
        @font-face { font-family: '等线'; font-weight: bold; src: local('PingFang SC Semibold'); }
        @font-face { font-family: 'DengXian'; font-weight: normal; src: local('PingFang SC'); }
        @font-face { font-family: 'DengXian'; font-weight: bold; src: local('PingFang SC Semibold'); }
        /* Right-side comment rail: only present in the DOM once the JS hook
           finds docx-preview comment nodes and moves them in. */
        .docx-comments-rail { position: fixed; top: 0; right: 0; bottom: 0;
                              width: 280px; overflow-y: auto; z-index: 50;
                              padding: 24px 16px; box-sizing: border-box;
                              background: #f3f4f6;
                              border-left: 1px solid rgba(0,0,0,0.1);
                              font: 13px -apple-system, sans-serif; }
        .docx-comments-rail > * { background: #fff; border-radius: 6px;
                                  padding: 10px 12px; margin-bottom: 12px;
                                  box-shadow: 0 1px 3px rgba(0,0,0,0.08); }
    </style>
    <script>\(jszipScript)</script>
    <script>\(docxPreviewScript)</script>
    </head>
    <body>
    <div id="status">加载中…</div>
    <div id="container"></div>
    <script>
    // Move the comment nodes docx-preview rendered into a right-side rail.
    // Builds no rail and no container when there are no comment nodes.
    function moveCommentNodesToRail(container) {
        var nodes = container.querySelectorAll(
            '.docx-comment, [class*="comment"], .docx-comments > *');
        if (!nodes || nodes.length === 0) return;
        var rail = document.querySelector('.docx-comments-rail');
        if (!rail) {
            rail = document.createElement('div');
            rail.className = 'docx-comments-rail';
            document.body.appendChild(rail);
        }
        for (var i = 0; i < nodes.length; i++) {
            rail.appendChild(nodes[i]);
        }
    }
    // Read the docx package's word/comments.xml in the normal render path,
    // using the JSZip the page already loads. docx-preview renders comments via
    // the CSS Custom Highlight API (display:none bubble, Safari 17+), which is
    // invisible on the minimum-supported macOS, so we parse the comments
    // ourselves rather than relying on docx-preview's comment nodes.
    //
    // For each w:comment in comments.xml we build a card carrying the comment's
    // body text (the concatenated w:t runs) and append it into the
    // .docx-comments-rail, creating the rail lazily on the first card.
    async function renderCommentsFromZip(bytes) {
        var zip = await JSZip.loadAsync(bytes);
        var commentsFile = zip.file('word/comments.xml');
        if (!commentsFile) return;
        var xml = await commentsFile.async('string');
        var doc = new DOMParser().parseFromString(xml, 'application/xml');
        var comments = doc.getElementsByTagName('w:comment');
        // No w:comment in the package: build no card and no rail, matching the
        // word/comments.xml-missing case above and moveCommentNodesToRail's
        // "no nodes, no rail" behavior.
        if (comments.length === 0) return;
        var rail = null;
        for (var i = 0; i < comments.length; i++) {
            var comment = comments[i];
            var runs = comment.getElementsByTagName('w:t');
            var text = '';
            for (var j = 0; j < runs.length; j++) {
                text += runs[j].textContent;
            }
            if (!rail) {
                rail = document.querySelector('.docx-comments-rail');
                if (!rail) {
                    rail = document.createElement('div');
                    rail.className = 'docx-comments-rail';
                    document.body.appendChild(rail);
                }
            }
            var author = comment.getAttribute('w:author');
            var date = comment.getAttribute('w:date');
            var card = document.createElement('div');
            if (author || date) {
                var meta = document.createElement('div');
                meta.className = 'docx-comment-meta';
                meta.textContent = [author, date].filter(Boolean).join(' · ');
                card.appendChild(meta);
            }
            var body = document.createElement('div');
            body.className = 'docx-comment-text';
            body.textContent = text;
            card.appendChild(body);
            rail.appendChild(card);
        }
    }
    (async function() {
        var status = document.getElementById('status');
        var container = document.getElementById('container');
        var b64 = "\(base64)";
        var bin = atob(b64);
        var bytes = new Uint8Array(bin.length);
        for (var i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
        var MIME = 'application/vnd.openxmlformats-officedocument.wordprocessingml.document';
        var opts = {
            className: 'docx', inWrapper: true, breakPages: true,
            ignoreLastRenderedPageBreak: true, experimental: true,
            trimXmlDeclaration: true, renderHeaders: true, renderFooters: true,
            renderFootnotes: true, renderEndnotes: true,
            renderComments: true, renderChanges: true,
        };
        try {
            await docx.renderAsync(new Blob([bytes], { type: MIME }), container, null, opts);
            moveCommentNodesToRail(container);
            await renderCommentsFromZip(bytes);
            status.textContent = '';
        } catch (e1) {
            // Retry after stripping OMML math (common pandoc-generated docx issue)
            try {
                status.textContent = '正在处理数学公式…';
                var zip = await JSZip.loadAsync(bytes);
                var docFile = zip.file('word/document.xml');
                if (!docFile) throw e1;
                var xml = await docFile.async('string');
                var cleaned = xml
                    .replace(/<m:oMathPara\\b[^>]*>[\\s\\S]*?<\\/m:oMathPara>/g,
                             '<w:p><w:r><w:t>[公式]</w:t></w:r></w:p>')
                    .replace(/<m:oMath\\b[^>]*>[\\s\\S]*?<\\/m:oMath>/g,
                             '<w:r><w:t>[公式]</w:t></w:r>');
                zip.file('word/document.xml', cleaned);
                var newBytes = await zip.generateAsync({ type: 'uint8array' });
                container.innerHTML = '';
                await docx.renderAsync(new Blob([newBytes], { type: MIME }), container, null, opts);
                moveCommentNodesToRail(container);
                status.textContent = '数学公式已转为占位符';
                setTimeout(function() { status.textContent = ''; }, 3000);
            } catch (e2) {
                status.className = 'error';
                status.textContent = '渲染失败: ' + (e2 && e2.message ? e2.message : e2);
            }
        }
    })();
    </script>
    </body>
    </html>
    """
}

/// Transforms `docmod read` HTML into HTML that places each comment in a
/// right-side sidebar.
///
/// `docmod read` emits comments as a plain `<aside data-type="comments">` block
/// at the bottom of the document body, with each entry shaped like
/// `<p data-id="cmN" data-author="..." data-date="...">text</p>`, plus inline
/// `<mark data-id="cmN">` anchors in the body. Left untouched, that block dumps
/// the author and text inline under the document text. This function pulls each
/// comment out of the `<aside>` block and rebuilds it as a card inside a
/// right-side sidebar container (`docmod-comments-rail`), each card carrying the
/// author and text and tied to its anchor id.
///
/// When the input has no `<aside data-type="comments">` block, the HTML is
/// returned unchanged, with no sidebar container.
///
/// The transform is intentionally string-based: docmod's read HTML has a
/// predictable shape, and a read-only display does not warrant pulling in an
/// XML parser. If a future docmod version changes the `<aside>`/`<mark>` shape,
/// this returns the document with no sidebar rather than failing loudly.
func transformDocmodComments(html: String) -> String {
    guard let asideRange = rangeOfCommentsAside(in: html) else {
        return html
    }

    let asideBlock = String(html[asideRange])
    let comments = parseDocmodComments(in: asideBlock)
    guard !comments.isEmpty else {
        return html
    }

    let cards = comments.map { comment -> String in
        """
        <div class="docmod-comment-card" data-comment-id="\(htmlEscape(comment.id))">
          <div class="docmod-comment-author">\(htmlEscape(comment.author))</div>
          <div class="docmod-comment-text">\(htmlEscape(comment.text))</div>
        </div>
        """
    }.joined(separator: "\n")

    let rail = """
    <aside class="docmod-comments-rail" data-type="comments">
    <style>
        /* Self-scoped right-side rail: a right-floated, fixed-width column.
           It does not pin to the viewport and emits no global `body` selector,
           so when the .doct path slices this block into its `.preview-frame`
           the rail stays contained in that frame instead of covering the doct
           header/metadata or shifting the whole doct page. */
        .docmod-comments-rail { float: right; width: 280px; margin-left: 24px;
                                box-sizing: border-box; padding: 16px;
                                background: #f3f4f6;
                                border: 1px solid rgba(0,0,0,0.1);
                                border-radius: 8px;
                                font: 13px -apple-system, sans-serif; }
        .docmod-comment-card { background: #fff; border-radius: 6px;
                               padding: 10px 12px; margin-bottom: 12px;
                               box-shadow: 0 1px 3px rgba(0,0,0,0.08); }
        .docmod-comment-card:last-child { margin-bottom: 0; }
        .docmod-comment-author { font-weight: 600; margin-bottom: 4px; }
        .docmod-comment-text { white-space: pre-wrap; }
    </style>
    \(cards)
    </aside>
    """

    // Replace the original plain `<aside>` block with the sidebar rail.
    return html.replacingCharacters(in: asideRange, with: rail)
}

/// A single comment parsed out of the `<aside data-type="comments">` block.
private struct DocmodComment {
    let id: String
    let author: String
    let text: String
}

/// Returns the range covering the whole `<aside data-type="comments"> ... </aside>`
/// block in `html`, or nil if there is none.
private func rangeOfCommentsAside(in html: String) -> Range<String.Index>? {
    guard let openStart = html.range(of: "<aside data-type=\"comments\"") else {
        return nil
    }
    guard let openTagEnd = html.range(of: ">", range: openStart.upperBound..<html.endIndex) else {
        return nil
    }
    guard let closeRange = html.range(of: "</aside>", range: openTagEnd.upperBound..<html.endIndex) else {
        return nil
    }
    return openStart.lowerBound..<closeRange.upperBound
}

/// Parses each `<p data-id=... data-author=...>text</p>` entry inside the
/// `<aside>` block into a `DocmodComment`.
private func parseDocmodComments(in asideBlock: String) -> [DocmodComment] {
    var comments: [DocmodComment] = []
    var searchStart = asideBlock.startIndex

    while let pOpenStart = asideBlock.range(of: "<p", range: searchStart..<asideBlock.endIndex),
          let pOpenEnd = asideBlock.range(of: ">", range: pOpenStart.upperBound..<asideBlock.endIndex),
          let pCloseStart = asideBlock.range(of: "</p>", range: pOpenEnd.upperBound..<asideBlock.endIndex) {

        let openTag = String(asideBlock[pOpenStart.lowerBound..<pOpenEnd.upperBound])
        let text = String(asideBlock[pOpenEnd.upperBound..<pCloseStart.lowerBound])
            .trimmingCharacters(in: .whitespacesAndNewlines)

        if let id = attributeValue("data-id", in: openTag),
           let author = attributeValue("data-author", in: openTag) {
            comments.append(DocmodComment(id: id, author: author, text: text))
        }

        searchStart = pCloseStart.upperBound
    }

    return comments
}

/// Extracts the value of `attribute="..."` from a tag string, or nil.
private func attributeValue(_ attribute: String, in tag: String) -> String? {
    guard let attrRange = tag.range(of: "\(attribute)=\"") else {
        return nil
    }
    guard let closeQuote = tag.range(of: "\"", range: attrRange.upperBound..<tag.endIndex) else {
        return nil
    }
    return String(tag[attrRange.upperBound..<closeQuote.lowerBound])
}

/// Minimal HTML text escaping for content placed into the sidebar cards.
private func htmlEscape(_ s: String) -> String {
    s.replacingOccurrences(of: "&", with: "&amp;")
     .replacingOccurrences(of: "<", with: "&lt;")
     .replacingOccurrences(of: ">", with: "&gt;")
     .replacingOccurrences(of: "\"", with: "&quot;")
}
