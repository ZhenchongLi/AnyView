// Markdown -> HTML rendering used by WebRenderer's preview.
//
// Single source of truth: this file is inlined into the WebRenderer HTML
// template (loaded via Bundle.module) AND loaded directly by the node test
// suite (Tests/WebRendererMdTests/md.test.js). Keep it framework-free so it
// runs both in the browser (WKWebView) and under node.
function md(s) {
    var mermaidBlocks = [];
    s = s.replace(/```(\w*)\n([\s\S]*?)```/g, function(_, lang, code) {
        if (lang === 'mermaid') {
            var idx = mermaidBlocks.length;
            mermaidBlocks.push(code.trim());
            return '<div data-mermaid-placeholder="' + idx + '"></div>';
        }
        var cls = lang ? ' class="language-' + lang + '"' : '';
        return '<pre><code' + cls + '>' + esc(code.trim()) + '</code></pre>';
    });
    s = s.replace(/^\|(.+)\|\n\|[-| :]+\|\n((?:\|.+\|\n?)*)/gm, function(_, header, body) {
        var ths = header.split('|').map(function(c){return '<th>'+c.trim()+'</th>';}).join('');
        var rows = body.trim().split('\n').map(function(r){
            return '<tr>'+r.replace(/^\||\|$/g,'').split('|').map(function(c){return '<td>'+c.trim()+'</td>';}).join('')+'</tr>';
        }).join('');
        return '<table><thead><tr>'+ths+'</tr></thead><tbody>'+rows+'</tbody></table>';
    });
    s = s.replace(/^######\s+(.*)$/gm, '<h6>$1</h6>');
    s = s.replace(/^#####\s+(.*)$/gm, '<h5>$1</h5>');
    s = s.replace(/^####\s+(.*)$/gm, '<h4>$1</h4>');
    s = s.replace(/^###\s+(.*)$/gm, '<h3>$1</h3>');
    s = s.replace(/^##\s+(.*)$/gm, '<h2>$1</h2>');
    s = s.replace(/^#\s+(.*)$/gm, '<h1>$1</h1>');
    s = s.replace(/^---+$/gm, '<hr>');
    s = s.replace(/^>\s+(.*)$/gm, '<blockquote>$1</blockquote>');
    s = s.replace(/\*\*\*(.+?)\*\*\*/g, '<b><i>$1</i></b>');
    s = s.replace(/\*\*(.+?)\*\*/g, '<b>$1</b>');
    s = s.replace(/\*(.+?)\*/g, '<i>$1</i>');
    s = s.replace(/`([^`]+)`/g, '<code>$1</code>');
    s = s.replace(/!\[([^\]]*)\]\(([^)]+)\)/g, '<img alt="$1" src="$2">');
    s = s.replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2">$1</a>');
    s = s.replace(/^[\-\*]\s+(.*)$/gm, '<li>$1</li>');
    s = s.replace(/((?:<li>.*<\/li>\n?)+)/g, '<ul>$1</ul>');
    s = s.replace(/^\d+\.\s+(.*)$/gm, '<li>$1</li>');
    s = s.replace(/^(?!<[hupoltbd]|<li|<bl|<hr|<im|<a )(.+)$/gm, '<p>$1</p>');
    s = s.replace(/<\/blockquote>\n<blockquote>/g, '<br>');
    s = s.replace(/<div data-mermaid-placeholder="(\d+)"><\/div>/g, function(_, idx) {
        return '<div class="mermaid">' + esc(mermaidBlocks[+idx]) + '</div>';
    });
    return s;
}
function esc(s) {
    return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');
}

if (typeof module !== 'undefined' && module.exports) {
    module.exports = { md: md, esc: esc };
}
