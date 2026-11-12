# Spine Workbench

Turn spine X-rays into a 3D spine you can measure.

Spine Workbench runs entirely on your own machine. There is no server, nothing is uploaded, and the app makes no network calls.

> **Early work in progress.** Spine Workbench is not a medical device and is not validated for clinical use. It shows previews with error bands. It does not predict any patient's outcome.

## What it does

1. **Films to points.** Load front, side and bending films, then watch the app find every vertebra from the top of the spine down. Correct anything it got wrong with a few clicks.
2. **Points to 3D.** Fit all the vertebrae at once into a 3D spine, with an error band on every measurement.
3. **Bones fitted accurately.** Shape real bone models to the fitted spine and check them against the films.

The first part is being built now.

## How it is built

- **Rust** does the computing, compiled to WebAssembly and run in background workers.
- **TypeScript and React** do everything you see and click.
- The same Rust also runs from the command line, so every calculation is tested on made-up spines with known answers before it touches a real film.
- On a real front film the vertebrae are found by a small trained network, written by hand in that Rust with its weights compiled in. How it was judged, and where it fails, is in [`engine/trained-finder/README.md`](engine/trained-finder/README.md).

## Run it

You need [rustup](https://rustup.rs), Node 22.13 or later and pnpm 9.12 or
later. Then, from the repository root:

```sh
cargo install wasm-bindgen-cli --version 0.2.130 --locked
pnpm install
pnpm dev
```

The first command installs the tool that turns the Rust engine into a
module the browser can load (its version must match the `wasm-bindgen`
crate; a bare version installs exactly that one). The pinned Rust
toolchain installs itself on the first cargo run. `pnpm dev` builds the
engine and serves the app at http://localhost:5173; open that address in
a browser.

The browser tests need Playwright's Chromium, installed once:

```sh
pnpm --filter ./proving-ground exec playwright install chromium
```

Other commands:

| Command | Does |
|---|---|
| `pnpm build` | the production app in `app/dist/` |
| `pnpm verify` | the checks CI runs: types and lints, unit tests, then browser tests on the production build, in Chromium only (CI adds WebKit) |
| `cargo spine <command>` | the same engine on the command line |

The production build is a static site. Serve it with the
`Content-Security-Policy` header set to `CONTENT_POLICY_HEADER` from
`app/build-plugins/content-policy.ts` on every file, as
`pnpm --filter ./app preview` does: the app's workers take their policy
from that header.

See [CONTRIBUTING.md](CONTRIBUTING.md) for how to work on it and
[MAP.md](MAP.md) for where everything is.

## Licence

Spine Workbench is released under the [Apache License 2.0](LICENSE).
