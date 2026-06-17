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

console.log('');
console.log(passed + ' passed, ' + failed + ' failed');
process.exit(failed === 0 ? 0 : 1);
