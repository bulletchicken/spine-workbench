# Contributing

Thank you for helping. This page is how work moves from an idea to
`main`. `MAP.md` tells you where things are; `docs/` tells you the rules
and why they were chosen.

## Setting up

You need:

- **rustup.** The pinned toolchain in `rust-toolchain.toml` (Rust
  1.99.1, with the `wasm32-unknown-unknown` target, rustfmt and clippy)
  installs itself the first time you run cargo. A Rust installed some
  other way may not be able to build for the browser.
- **The wasm-bindgen command-line tool**, at exactly the version of the
  `wasm-bindgen` crate in `Cargo.toml`:
  `cargo install wasm-bindgen-cli --version 0.2.130 --locked` (a bare
  version installs exactly that one).
  `cargo chores versions` checks they match.
- **Node 22.13 or later and pnpm 9.12 or later.** `.npmrc` sets
  engine-strict, so an older Node is refused.

Then:

```sh
pnpm install     # also points git at chores/git-hooks, for the commit-msg hook
pnpm verify      # everything CI runs, in order
```

The browser tests run in Chromium by default, so install it once before
the first `pnpm verify`:
`pnpm --filter ./proving-ground exec playwright install chromium`. To add
WebKit, install it the same way (`playwright install webkit`) and set
`BROWSERS=chromium,webkit`. CI always runs both, so a local `pnpm verify`
covers everything CI does except WebKit.

## The checks, in order

| Command | Runs |
|---|---|
| `pnpm check` | `cargo fmt --check`, clippy with warnings as errors, `cargo chores seams`, `cargo chores shapes --check`, `cargo chores map --check`, the bridge checked for wasm32, `tsc`, and ESLint with the network ban and import boundaries |
| `pnpm test` | the release engine, `cargo test --workspace`, then Vitest in `app/` and `proving-ground/` (the parity tests run the real engine in Node) |
| `pnpm browser` | the production build, a scan of it, then Playwright against `vite preview` with the content policy |
| `pnpm verify` | all three, in that order |

Run them before you open a pull request, and say in the pull request
which you ran.

## Branches

One branch and one pull request per slice of work, named
`w<wave>/<nn>-<short-name>`, for example `w1/13-film-reader`. Branch from
`main`; keep the branch rebased on it.

## Commits

Small commits, one concern each, and each one leaves the repository
passing every check. The subject reads

```
type(scope): imperative summary
```

- **type** is one of `feat`, `fix`, `refactor`, `perf`, `test`, `docs`,
  `build`, `ci`, `chore`, `style`;
- **scope** is optional. When given it is `deps` or a real folder: a
  top-level folder (`app`, `engine`, `chores`, `docs`, `proving-ground`),
  an engine crate (`film-reader`), an area of `app/src` (`engine-room`),
  or a step named without its number (`find`);
- the whole subject is at most 72 characters, with no full stop.

Leave a blank line, then a body wrapped at 72 columns saying **why**. For
example:

```
feat(film-reader): drop every DICOM tag not on the keep-list

The keep-list is the only place a tag value leaves the parser, so a new
tag can never reach the case file by accident.
```

Commits carry one author and no trailers or tool footers: no
`Co-authored-by`, `Signed-off-by` or "Generated with" lines.
`chores/check-commit-message.sh` checks all of this. The commit-msg hook
in `chores/git-hooks/` runs it on every commit you make, and CI runs it on
every commit in a pull request and on the pull request's own text.

## Pull requests

Use the template: the slice, what changed, why, how it was checked (the
commands, in order), captures for anything visual, whether a contract
changed, and anything new a contributor's machine must download, with its
size.

**Look before you say it works.** For anything visible, open the app in a
browser before marking the pull request ready: `pnpm dev` for work against
the stand-in engine, `pnpm build` and `pnpm --filter ./app preview` for
work against the real engine.

**Review.** Someone other than the author reviews the diff and posts a
review comment. The author folds each fix into the commit it belongs to,
so that every commit reaching `main` still stands on its own:

```sh
git add <the files you fixed>
git commit --fixup=<sha>
git fetch origin
git rebase -i --autosquash origin/main
git push --force-with-lease
```

Then post a comment saying what changed for each point, so the review
can follow. The commit-msg hook lets `fixup!`, `squash!` and `amend!`
commits through, because the rebase folds them away; CI refuses any
that reach a pull request. When the reviewer posts "Reviewed: ready to
merge" and every workflow is green, merge with

```sh
gh pr merge --rebase --delete-branch
```

Rebase merging keeps every small commit on `main`.

## Owning paths

Each slice owns a set of paths and touches nothing else. Reading is never
restricted, and importing a frozen contract is always allowed.

## Dependency pull requests

Need a new crate or package? Open `chore(deps): add <name> for <why>` on
its own, with its size, and merge it before the work that uses it. Every
Rust dependency is pinned once, exactly, in the workspace `Cargo.toml`;
every package is pinned exactly in its `package.json`. A lockfile conflict
is resolved by regenerating the lockfile, never by hand.

## Contract-change pull requests

The frozen contracts are listed in `docs/contracts/` and in the pull
request template. To change one, open a small pull request that touches
only the contract and its regenerated output (`cargo chores shapes` for
the shapes), additive where possible. The exhaustive switches over error
codes, frame kinds and check kinds then show every other slice what to
update.

## What never goes in

- **Anything that identifies a patient.** Real films, datasets and
  hand-marked cases live in `local-only/`, which git ignores. Tests write
  their own fixtures or use made-up spines.
- **Network calls.** The app makes none. The content policy, the lint
  rule and the browser tests enforce it; none of the three is ever
  loosened to make something pass.
- **Claims about outcomes.** The app shows previews with error bands. It
  never claims to predict a patient's outcome, and it is not a medical
  device. `app/src/words/never-claim.test.ts` checks the wording.
- **Sources we cannot cite.** A number the app relies on cites its public
  source by PMID or DOI.
