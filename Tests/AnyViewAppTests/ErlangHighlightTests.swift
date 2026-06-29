import XCTest
@testable import AnyViewApp

final class ErlangHighlightTests: XCTestCase {
    // Acceptance criterion #1 (issue #23): the highlight.js Erlang grammar must be
    // shipped as its own resource file in the macOS resource bundle, readable from
    // Bundle.module, with non-empty content. WebRenderer.hljsErlangScript is the
    // static property that reads `hljs-erlang.js` out of Bundle.module (mirroring
    // hljsLatexScript). Reading it non-empty proves both that the resource was
    // bundled and that its content is non-empty.
    func test_hljsErlangScript_isBundledAndNonEmpty() {
        XCTAssertFalse(
            WebRenderer.hljsErlangScript.isEmpty,
            "WebRenderer.hljsErlangScript must read a non-empty hljs-erlang.js out of Bundle.module; an empty value means the Erlang grammar resource was not bundled"
        )
    }

    // Acceptance criterion #2 (issue #23): the source-view HTML WebRenderer
    // generates for `.erl` files must contain the Erlang grammar definition.
    // loadCodeFile builds that HTML and hands it straight to loadHTMLString, so
    // the testable surface is the pure builder buildCodeHTML it delegates to
    // (mirroring how loadDocxContent delegates to buildDocxHTML). For an Erlang
    // file (lang == "erlang") the built HTML must carry both the characteristic
    // substring of the Erlang grammar script (hljs.registerLanguage("erlang")
    // and the <code class="language-erlang"> hook that hljs.highlightAll() needs
    // to match the registered language.
    func test_buildCodeHTML_injectsErlangGrammarForErlFiles() {
        let erlangGrammarStub = "/*stub*/ if(window.hljs){hljs.registerLanguage(\"erlang\",erlang);}"
        let html = buildCodeHTML(
            escaped: "-module(demo).",
            lineCount: 1,
            lang: "erlang",
            escapedFilename: "demo.erl",
            highlightScript: "/* stub hljs core */",
            erlangGrammarScript: erlangGrammarStub
        )
        XCTAssertTrue(
            html.contains("hljs.registerLanguage(\"erlang\""),
            "Erlang source-view HTML must inject the Erlang grammar definition (the registerLanguage(\"erlang\" call) so hljs.highlightAll() can highlight .erl files"
        )
        XCTAssertTrue(
            html.contains("<code class=\"language-erlang\">"),
            "Erlang source-view HTML must mark the code block with class=\"language-erlang\" so the registered grammar is matched"
        )
    }
}
