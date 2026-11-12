# proving-ground

How we know the engine and the app work. Every "how do we know?" lands
here, in four rungs, each one closer to real use:

1. **Known answers.** Small hand-checked fixtures that Rust and TypeScript
   both read: `known-answers/case-files/` holds valid and invalid case
   files, and `verdicts.json` says what each must do;
   `known-answers/side-swap.json` gives a film's marks before and after
   their left/right names are swapped for a new facing.
2. **Made-up spines.** Spines and films generated with their answers
   known, scored against thresholds in CI (once the scorekeeper's suites
   exist).
3. **Real films.** Scored on this machine only, from datasets kept in
   `local-only/`; the reports are committed under `scoreboards/` with the
   git commit and a hash of the dataset listing (once real-film scoring
   exists).
4. **The browser.** The production build, under its content policy, in
   Chromium and WebKit.

## What is here today

| Path | Proves |
|---|---|
| `known-answers/case-files/` | strict reading: every file and the verdict it must get |
| `known-answers/spine-3d/` | the 3D view places a bone's body where the engine's forward model puts the vertebra's: endplate centres and body corners in the patient and on four films, read by `app/src/spine-3d/placing/known-answers.test.ts` |
| `known-answers/side-swap.json` | swapping a film's left/right (or anterior/posterior) names keeps every point's position and flags; read by `engine/shapes/tests/side_swap.rs` and, once it exists, the case reducer's test |
| `parity/contract.test.ts` | the release engine runs in Node, reports the app's shapes fingerprint, and every golden case file crosses into JavaScript and back unchanged |
| `parity/trained-pass.test.ts`, `known-answers/trained-pass/` | the trained finder in the release engine gives exactly the native engine's answer on a made-up front film, as front-film marks the strict reader takes, names flagged as estimates; the native answer is held to the same file by `engine/trained-finder/tests/finds_the_vertebrae_on_a_made_up_film.rs` |
| `in-the-browser/no-network/nothing-leaves.spec.ts` | the production build with the real engine, offline from empty Films through the made-up case, Find, review, both moments at 640 × 800 and Start over: every request is to a built file, exactly one fetch (the engine), no WebSocket and no policy violation in the page or any worker |
| `in-the-browser/no-network/worker-violations-are-seen.spec.ts` | the policy recorder sees a worker reaching outside from its very first line, so the spec above cannot pass a leaking worker |
| `in-the-browser/no-network/content-policy.spec.ts` | the exact policy in the page and in every file's header; eval blocked in the page and the workers; the engine compiles; an outside fetch is refused by the policy itself; no blob: workers; no stand-in code shipped |
| `in-the-browser/no-network/no-outside-urls-in-build.test.ts` | every web address written in the build is on a short reviewed list |
| `in-the-browser/test-rig/` | what the browser tests are mounted on: the request ledger, the policy-violation recorder (page and workers), the build's file list and the wait for the records to hold still |

## Running it

```sh
pnpm test      # builds the engine, then the parity tests (with the Rust and app tests)
pnpm browser   # builds the app, scans the build, then Playwright
```

Browser tests run only against `vite preview` of the production build,
because development mode has no content policy. Locally they run in
Chromium; set `BROWSERS=chromium,webkit` to add WebKit once its browser
is installed (`pnpm --filter ./proving-ground exec playwright install
webkit`). CI runs both.
