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
            } catch (e2) {
                status.className = 'error';
                status.textContent = '无法渲染文档';
            }
        }
    })();
    </script>
    </body>
    </html>
    """
}
