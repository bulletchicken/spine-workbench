# Naming in code

Names should let you guess a path before you search for it. Plain words
first; a playful name only when its meaning is still obvious
(`darkroom` for image filters, `proving-ground` for proof).

## Rules

| Thing | Rule | Example |
|---|---|---|
| Folders | kebab-case | `engine-room/`, `made-up-spines/` |
| Rust module folders and files | snake_case, as Rust requires | `boundary_wand/renumber.rs` |
| Rust crates | package name = folder name, `publish = false` | `film-reader` |
| React components | PascalCase `.tsx` | `LeftRightConfirm.tsx` |
| Hooks | `use-*.ts(x)` | `use-case-checks.ts` |
| Other TypeScript | kebab-case `.ts` | `frame-player.ts` |
| Unit tests | beside the code: Rust `#[cfg(test)]`, TypeScript `*.test.ts` | `apply-edit.test.ts` |
| Rust integration tests | a sentence saying what holds | `pins_hold.rs` |
| Browser tests | `*.spec.ts` | `nothing-leaves.spec.ts` |
| Generated code | inside `generated/` or `wasm-build/`, never edited by hand | `app/src/shapes/generated/` |
| Enum values in JSON | kebab-case | `'not-visible'`, `'film-unsupported-compression'` |
| JSON fields | camelCase | `mmPerPx` |

## Words we always use

- **Film kinds** are `front`, `side`, `bend-left` and `bend-right`
  everywhere: Rust, TypeScript, JSON and file names inside the case zip
  (`films/bend-left.png`). Bending films are front views.
- **Mark names** are fixed; see [mark and level names](mark-and-level-names.md).
- **Spelling** is Canadian in our own names: centre, colour, grey,
  normalise, metre. Names that belong to a platform or library stay as
  they are (CSS `color`, `GrayImage` in someone else's crate).
- One word per idea: a *film* (not image or radiograph) in names, a
  *level* for T1 to L5, a *vertebra* for one bone.

## Folders that are never made

No folder is called `utils`, `helpers`, `common`, `shared`, `misc` or
`lib`. If code does not obviously belong somewhere, that is a sign its
name is wrong, not that it needs a bin. Two folders hold small general
pieces and stay narrow on purpose: `app/src/building-blocks/` (UI pieces
used by more than one step) and `chores/` (repository upkeep, one job per
file).

## Two vocabularies, one mapping

The engine names things by **what they compute** (nouns); the app's step
folders by **what the surgeon does** (verbs). Within each side a name
never changes spelling:

- engine crate `film-reader` = its export file
  `engine/wasm-bridge/src/exports/film_reader.rs` = its row in `MAP.md`;
- step folder `3-find-vertebrae/` = its words file `words/find-vertebrae.ts`
  = its stand-in script `stand-in/scripts/find-vertebrae.ts`.

`toolbox` and `made-up-spines` are the same word on both sides: the engine
computes them, the app lets you use them.

## Guessing a path

For an engine feature in crate `C`: the Rust is in `engine/C/src/`, its
door to the browser is `engine/wasm-bridge/src/exports/C.rs` (with
underscores), its jobs are in
`app/src/engine-room/workers/<film|spine>-worker/jobs/`, and screens call
it through `app/src/engine-room/engine-client.ts`. A screen lives in
`app/src/steps/<n>-<verb>/` with its words in `app/src/words/<verb>.ts`.
Proof is always under `proving-ground/`. `MAP.md` at the root lists the
rest.
