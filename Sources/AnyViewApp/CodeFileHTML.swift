import Foundation

/// Builds the HTML string the source-code path hands to the web view.
///
/// The highlight.js core script and the optional Erlang grammar script are
/// passed in as parameters so this function performs no bundle lookups and can
/// be called from a unit test with stub scripts, without constructing
/// `WebRenderer`, any `NSView`, or a `WKWebView`. Mirrors `buildDocxHTML`.
func buildCodeHTML(
    escaped: String,
    lineCount: Int,
    lang: String,
    escapedFilename: String,
    highlightScript: String,
    erlangGrammarScript: String
) -> String {
    let langClass = lang.isEmpty ? "" : "language-\(lang)"
    let highlightInline = highlightScript.isEmpty ? "" : "<script>\(highlightScript)</script>"
    // Inject the Erlang grammar only for Erlang sources, after highlightInline so
    // hljs is already loaded before the grammar registers itself (mirrors how
    // loadTexFile injects latexGrammar after highlightInline). The core
    // highlight.min.js build ships without Erlang, so .erl files need it appended.
    let erlangGrammar = (lang == "erlang" && !erlangGrammarScript.isEmpty)
        ? "<script>\(erlangGrammarScript)</script>"
        : ""

    return """
    <!DOCTYPE html>
    <html>
    <head>
    <meta charset="UTF-8">
    <meta name="color-scheme" content="light dark">
    <style>
        body { margin: 0; padding: 20px 24px;
               font-family: "SF Mono", Menlo, Consolas, monospace; font-size: 13px;
               background: #f8f9fa; color: #1a1a1a; }
        pre { margin: 0; line-height: 1.5; white-space: pre-wrap; word-wrap: break-word;
              tab-size: 4; }
        pre code { font-family: inherit; font-size: inherit; }
        .header { color: #888; font-size: 11px; margin-bottom: 12px; padding-bottom: 8px;
                  border-bottom: 1px solid #ddd; }
        .hljs{display:block;overflow-x:auto;padding:0;color:#333;background:transparent;}
        .hljs-comment,.hljs-quote{color:#998;font-style:italic;}
        .hljs-keyword,.hljs-selector-tag,.hljs-subst{color:#333;font-weight:bold;}
        .hljs-number,.hljs-literal,.hljs-variable,.hljs-template-variable,.hljs-tag .hljs-attr{color:#008080;}
        .hljs-string,.hljs-doctag{color:#d14;}
        .hljs-title,.hljs-section,.hljs-selector-id{color:#900;font-weight:bold;}
        .hljs-subst{font-weight:normal;}
        .hljs-type,.hljs-class .hljs-title{color:#458;font-weight:bold;}
        .hljs-tag,.hljs-name,.hljs-attribute{color:#000080;font-weight:normal;}
        .hljs-regexp,.hljs-link{color:#009926;}
        .hljs-symbol,.hljs-bullet{color:#990073;}
        .hljs-built_in,.hljs-builtin-name{color:#0086b3;}
        .hljs-meta{color:#999;font-weight:bold;}
        .hljs-deletion{background:#fdd;}.hljs-addition{background:#dfd;}
        @media (prefers-color-scheme: dark) {
            body { background: #1e1e1e; color: #d4d4d4; }
            .header { border-bottom-color: #333; }
            .hljs{color:#abb2bf;background:transparent;}
            .hljs-comment,.hljs-quote{color:#5c6370;font-style:italic;}
            .hljs-doctag,.hljs-keyword,.hljs-formula{color:#c678dd;}
            .hljs-section,.hljs-name,.hljs-selector-tag,.hljs-deletion,.hljs-subst{color:#e06c75;}
            .hljs-literal{color:#56b6c2;}
            .hljs-string,.hljs-regexp,.hljs-addition,.hljs-attribute,.hljs-meta-string{color:#98c379;}
            .hljs-built_in,.hljs-class .hljs-title{color:#e6c07b;}
            .hljs-attr,.hljs-variable,.hljs-template-variable,.hljs-type,.hljs-selector-class,
            .hljs-selector-attr,.hljs-selector-pseudo,.hljs-number{color:#d19a66;}
            .hljs-symbol,.hljs-bullet,.hljs-link,.hljs-meta,.hljs-selector-id,.hljs-title{color:#61aeee;}
        }
    </style>
    \(highlightInline)
    \(erlangGrammar)
    </head>
    <body>
    <div class="header">\(lineCount) lines · \(escapedFilename)</div>
    <pre><code class="\(langClass)">\(escaped)</code></pre>
    <script>if(window.hljs){hljs.highlightAll();}</script>
    </body>
    </html>
    """
}
