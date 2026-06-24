// Tests for the extracted markdown renderer md() used by WebRenderer.
//
// Run: node Tests/WebRendererMdTests/md.test.js
//
// Loads the single source of truth (Sources/AnyViewApp/Resources/markdown.js)
// so the template and the test can't drift.
'use strict';

const path = require('path');
const assert = require('assert');

const { md } = require(
    path.join(__dirname, '..', '..', 'Sources', 'AnyViewApp', 'Resources', 'markdown.js')
);

let passed = 0;
let failed = 0;

function test(name, fn) {
    try {
        fn();
        passed += 1;
        console.log('ok   - ' + name);
    } catch (err) {
        failed += 1;
        console.log('FAIL - ' + name);
        console.log('       ' + (err && err.message ? err.message : err));
    }
}

function countOccurrences(haystack, needle) {
    return haystack.split(needle).length - 1;
}

test('test_go_block_with_blank_line_yields_single_pre_code', function() {
    const input = '```go\nfunc main() {\n\n\tprintln("hi")\n}\n```';
    const out = md(input);
    // Exactly one intact <pre><code>...</code></pre> element. The interior
    // ([\s\S]*? up to the first </code>) must not have been fractured by the
    // paragraph regex injecting <p> tags into the code block.
    const matches = out.match(/<pre><code[^>]*>([\s\S]*?)<\/code><\/pre>/g) || [];
    assert.strictEqual(
        matches.length,
        1,
        'expected exactly one <pre><code>...</code></pre> element, got ' +
            matches.length + ': ' + out
    );
    const interior = matches[0].replace(/^<pre><code[^>]*>/, '').replace(/<\/code><\/pre>$/, '');
    assert.ok(
        interior.indexOf('<p>') === -1 && interior.indexOf('</p>') === -1,
        'code block element must not contain injected <p> tags: ' + out
    );
});

test('test_blank_line_inside_code_block_preserved', function() {
    // A blank line between two code lines must survive in md()'s output as a
    // blank line inside the <pre><code> interior, not be swallowed/collapsed.
    const input = '```go\nfunc main() {\n\n\tprintln("hi")\n}\n```';
    const out = md(input);
    const matches = out.match(/<pre><code[^>]*>([\s\S]*?)<\/code><\/pre>/) || [];
    assert.ok(matches.length > 0, 'expected a <pre><code> element: ' + out);
    const interior = matches[1];
    assert.ok(
        interior.indexOf('\n\n') !== -1,
        'blank line inside code block must be preserved as a blank line: ' +
            JSON.stringify(interior)
    );
});

test('test_code_block_content_has_no_injected_tags', function() {
    // The <pre><code>...</code></pre> substring of md()'s output must not
    // contain a <p> tag injected by the paragraph/inline regexes.
    const input = '```go\nfunc main() {\n\n\tprintln("hi")\n}\n```';
    const out = md(input);
    const start = out.indexOf('<pre><code');
    const end = out.indexOf('</code></pre>', start);
    assert.ok(start !== -1 && end !== -1, 'expected a <pre><code> element: ' + out);
    const block = out.slice(start, end + '</code></pre>'.length);
    assert.ok(
        block.indexOf('<p>') === -1,
        'code block must not contain injected <p> tags: ' + JSON.stringify(block)
    );
});

test('test_soft_wrapped_paragraph_lines_join_as_space', function() {
    // CommonMark: a single newline inside a paragraph is a space, not a <br>.
    // Multi-line paragraphs from source files wrapped at ~80 chars must reflow.
    const input = 'First line of paragraph\nsecond line of paragraph\nthird line.\n\nNew paragraph.';
    const out = md(input);
    assert.ok(
        out.indexOf('<p>First line of paragraph second line of paragraph third line.</p>') !== -1,
        'soft-wrapped lines must join into one <p>: ' + out
    );
    assert.ok(
        out.indexOf('<p>New paragraph.</p>') !== -1,
        'blank-line-separated paragraph must remain separate: ' + out
    );
    assert.ok(
        out.indexOf('<p>second line') === -1,
        'second line must not become its own <p>: ' + out
    );
});

test('test_soft_wrap_joins_line_starting_with_inline_tag', function() {
    // A paragraph line that starts with an inline element (<a>, <code>) after
    // inline processing must still be joined with the preceding line.
    // Regression: the original fix treated any "<"-starting line as a block boundary.
    const input = 'Visit [example](http://example.com) and serves as\nthe barrier.';
    const out = md(input);
    assert.ok(
        !/<p>the barrier\.?<\/p>/.test(out),
        'line starting with plain text after a link line must join: ' + out
    );
    // paragraph starting mid-line with a link followed by continuation
    const input2 = 'Prefix text\n[link](http://x.com) continuation text';
    const out2 = md(input2);
    assert.ok(
        out2.indexOf('Prefix text') !== -1 && out2.indexOf('continuation text') !== -1,
        'link-starting continuation line must join: ' + out2
    );
    const count2 = (out2.match(/<p>/g) || []).length;
    assert.strictEqual(count2, 1, 'must produce exactly one <p>: ' + out2);
});

test('test_soft_wrap_does_not_merge_across_block_elements', function() {
    // A plain-text line must not be joined with an adjacent heading or list.
    const input = '# Heading\n\nParagraph line one\nparagraph line two.\n\n- item';
    const out = md(input);
    assert.ok(out.indexOf('<h1>Heading</h1>') !== -1, 'heading must survive: ' + out);
    assert.ok(
        out.indexOf('<p>Paragraph line one paragraph line two.</p>') !== -1,
        'paragraph lines must join: ' + out
    );
    assert.ok(/<li>item<\/li>/.test(out), 'list item must survive: ' + out);
});

test('test_mixed_document_keeps_h1_ul_table_p', function() {
    // A document mixing a heading, an unordered list, a table and a plain
    // paragraph must still render each element to its own block: the
    // code-block placeholder change must not have broken the other regexes.
    const input = [
        '# Title',
        '',
        '- one',
        '- two',
        '',
        '| A | B |',
        '|---|---|',
        '| 1 | 2 |',
        '',
        'Just a paragraph.'
    ].join('\n');
    const out = md(input);
    assert.ok(out.indexOf('<h1>Title</h1>') !== -1,
        'heading must render to <h1>: ' + out);
    assert.ok(/<ul>\s*<li>one<\/li>/.test(out),
        'unordered list must render to <ul><li>: ' + out);
    assert.ok(out.indexOf('<table>') !== -1,
        'table must render to <table>: ' + out);
    assert.ok(out.indexOf('<p>Just a paragraph.</p>') !== -1,
        'plain paragraph must render to <p>: ' + out);
});

test('test_inline_code_escapes_html_tags', function() {
    // Inline code spans whose content is HTML (common in docs about markup)
    // must be escaped, otherwise raw block tags like <p> leak into the DOM,
    // get hoisted out of surrounding table cells and shred the layout.
    const out = md('Use `<p data-id="p3">x</p>` here.');
    assert.ok(
        out.indexOf('<code>&lt;p data-id="p3"&gt;x&lt;/p&gt;</code>') !== -1,
        'inline code content must be HTML-escaped: ' + out
    );
    assert.ok(
        out.indexOf('<code><p') === -1,
        'raw block tag must not leak out of inline code: ' + out
    );
});

test('test_table_cell_with_html_code_stays_intact', function() {
    // A table cell containing an HTML snippet in inline code must keep the
    // table well-formed: no raw <p>/<td> tags injected from the cell content.
    const input = [
        '| 能力 | 写法 |',
        '|---|---|',
        '| 替换 | `<p data-id="p3">新文本</p>` |'
    ].join('\n');
    const out = md(input);
    assert.ok(out.indexOf('<table>') !== -1, 'table must render: ' + out);
    assert.ok(out.indexOf('<code><p') === -1,
        'cell HTML must be escaped, not leaked: ' + out);
});

console.log('');
console.log(passed + ' passed, ' + failed + ' failed');
process.exit(failed === 0 ? 0 : 1);
