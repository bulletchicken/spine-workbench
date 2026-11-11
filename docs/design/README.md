# design

What the app looks like, says and does, and how it moves. The design is
called Clinical White: a white, ruled page, four films on a bench, each
on black, the spine unrolled between them as a ladder, one sentence and
one ink button.

| File | What it is |
|---|---|
| [`design-language.md`](design-language.md) | The rules: the concept, the shell, every state of the flow, tokens, type, motion, components, wording, accessibility and build notes. |
| [`mock.html`](mock.html) | The design, working. One self-contained file: it makes no request and holds no patient image. Open it in a browser. |
| [`tokens.css`](tokens.css) | The design tokens alone, each ink and colour with a value for paper and a value for a film. The app's copy is `app/src/look-and-feel/tokens.css`; a test holds the two to the same values. |
| [`where-the-app-differs.md`](where-the-app-differs.md) | The short list of where the app departs from the mock on purpose. |

Where the page and the mock disagree, the mock wins for how something
looks and moves, and the page wins for rules, names and wording.

## Looking at the two side by side

The mock opens in any state by name, and the app's gallery shows every
part in every state:

```sh
open docs/design/mock.html             # then add #s=pass-result, #s=review-split, …
pnpm dev                               # then open http://localhost:5173/?gallery
```

Compare at 1440 by 900 and at 1024 wide, which sit either side of the
one breakpoint. The mock runs on system fonts and the app ships its own,
so line breaks differ a little; everything else should match, or be on
the list of differences.
