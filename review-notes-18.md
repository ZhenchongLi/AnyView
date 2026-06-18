### Claude

## Verdict
approve

## Real issues
None.

## Questions
- The math-fallback retry path (`WordCommentHTML.swift:300-317`) calls `moveCommentNodesToRail` but never calls `renderCommentsFromZip`, `highlightCommentRanges`, or `positionCardsByHighlight`. A docx that trips the OMML strip gets zero comment cards and zero highlights. The missing `renderCommentsFromZip` predates this branch, so the cards were already gone on that path. This branch just leaves the new highlight/position pass out of it too. Is the fallback path meant to stay feature-less, or should it carry the same three calls the primary path now runs?
- `positionCardsByHighlight` reads `span.offsetTop` inside `#container` and writes it straight into `card.style.transform = translateY(offsetTop)`. The rail is `position: fixed; padding: 24px` (`WordCommentHTML.swift:51`), a different coordinate origin from `#container`, which scrolls. The card's translate is measured from the rail's padded top, not the page. Cards will sit at the right relative order but not pixel-aligned to the passage, and they won't track when the body scrolls. The design names this as manual-verification-only and accepts it, so it's not a blocker — confirming it's the known trade-off, not an oversight.

## Nits
- `highlightCommentRanges` iterates `for (var id in starts)` — `for...in` walks the prototype chain. Object literal `{}` is fine here, but `Object.keys(starts)` reads cleaner and can't surprise.
- The card click handler and span click handler both build a `querySelector` string by concatenating `cid` into an attribute selector. Comment ids are numeric (`w:id`), so injection isn't reachable, but a stray non-numeric id would throw inside the handler. Low stakes.

## Functional evidence
- Criterion 1 — pass: `WordCommentHTML.swift:132` reads `comment.getAttribute('w:id')`; `:135` sets `card.setAttribute('data-comment-id', commentId)`. Verified by `test_buildDocxHTML_cardCarriesCommentId` (asserts `w:id` and `data-comment-id` in the comment-parsing region).
- Criterion 2 — pass: `WordCommentHTML.swift:64` `span[data-comment-id] { background: rgba(255, 213, 79, 0.45); ... }` inside the `<style>` block. Verified by `test_buildDocxHTML_highlightSpanHasBackgroundCss`.
- Criterion 3 — pass: `highlightCommentRanges` (`WordCommentHTML.swift:175-218`) walks `#container` with a `NodeFilter.SHOW_COMMENT` TreeWalker, regex-matches `commentRangeStart` / `commentRangeEnd` + id, and moves the nodes between the pair into `<span data-comment-id="...">`. Verified by `test_buildDocxHTML_wrapsCommentRangeInSpan`.
- Criterion 4 — pass: `positionCardsByHighlight` (`WordCommentHTML.swift:236-262`) reads `span.offsetTop` and writes `card.style.transform = 'translateY(' + top + 'px)'`. Verified by `test_buildDocxHTML_positionsCardByOffsetTop`.
- Criterion 5 — pass: card-side `card.addEventListener('click', ...)` (`WordCommentHTML.swift:137-146`) looks up the paired `span[data-comment-id]` and calls `span.scrollIntoView`. Verified by `test_buildDocxHTML_cardClickScrollsToHighlight`.
- Criterion 6 — pass: span-side `span.addEventListener('click', ...)` (`WordCommentHTML.swift:184-194`) finds the paired card and calls `card.scrollIntoView` + `card.classList.add('docx-comment-emphasis')`. Verified by `test_buildDocxHTML_highlightClickEmphasizesCard`.
- Criterion 7 — pass: `positionCardsByHighlight` sorts `placed` by target top, tracks `prevBottom = top + card.offsetHeight`, and pushes a later card down when `entry.top < prevBottom` (`WordCommentHTML.swift:253-260`). Verified by `test_buildDocxHTML_stacksOverlappingCards`.
