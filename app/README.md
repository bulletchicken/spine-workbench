# app

Everything you see and click, in TypeScript and React. The computing
happens in the Rust engine, which runs in background workers; the app
reaches it only through the engine room.

## Commands

- `pnpm dev`: development, with the stand-in engine; add `?engine=real` to read your own films.
- `pnpm build`: production, with the real engine and no films from `src/sample-films/on-this-machine/`.
- `pnpm --filter ./app preview`: serve the production build with the content policy header.
- `pnpm --filter ./app films-on-this-machine`: deliberately build with local sample films into
  the ignored `app/films-on-this-machine/dist/` folder, then serve that copy on this computer with
  the same content policy header. Never publish that folder.

Real films need the real engine in development (`/?engine=real`); the stand-in reads only its own.

Saved cases open through the film picker or a drop; Save case appears after vertebrae are found. Version 1 keeps films, marks, level names and facing; review looks start again on reopen.

## Folders

| Folder | Holds |
|---|---|
| `src/steps/` | the moments, numbered in the order they are used: `1-films`, `2-find` |
| `src/app-shell/` | the frame round every moment: the bar, the step control, the stage, the dock, the notice |
| `src/guide/` | the dock's one sentence and its one button |
| `src/bench/` | the stage: the films in their places, and how they move between moments |
| `src/film-viewer/` | one film on screen: pixels on the GPU, layers over them, its camera |
| `src/spine-3d/` | the spine in three dimensions: bone shapes placed by a fit, with the films it came from; not mounted yet |
| `src/marks/` | what is drawn on a film: outlines, their names, the travelling light |
| `src/ladder/` | the unrolled spine between the Front and Side films |
| `src/auto-pass/` | the automatic pass, as something to watch: its recording, clock and layers, and the replay of a finished search |
| `src/toolbox/` | the adjusting tools you pick up, the tool dock, the magnifier and the ruler |
| `src/review/` | what needs a person, worked out from the case as pure functions |
| `src/on-this-device/` | the badge and popover that show nothing leaves this computer |
| `src/case-sheet/` | saving the case as one file |
| `src/reopen-case/` | a saved case, read strictly and hung again |
| `src/made-up-spines/` | the made-up sample case |
| `src/engine-room/` | the loader, the workers, their messages and the engine client |
| `src/case-state/` | the open case (what gets saved) and its undo history |
| `src/view-state/` | what you are looking at right now (never saved) |
| `src/open-and-save/` | files in and out, with no network and no screens |
| `src/shapes/` | the shared types, generated from the Rust shapes |
| `src/look-and-feel/` | the design system's two stylesheets, what the app adds beside them, and the signs it draws |
| `src/choreography/` | how things move: shared keyframes and the one animation loop |
| `src/building-blocks/` | small UI pieces more than one area uses, and the one key listener |
| `src/words/` | every sentence the user reads |
| `src/sample-case/` | dev and tests only: the one fixed story every screen can be opened in |
| `src/sample-films/` | real front films kept on this machine only; included in development and the deliberate local build, excluded from production; never committed |
| `src/gallery/` | dev only: every part in every state, at `?gallery` |
| `build-plugins/` | the content policy and the engine build, as Vite plugins |

## Who may import whom

ESLint enforces these rules (`eslint.config.js`, the `AREAS` list).

```
  app-shell                     nothing banned
      │
      ▼
  steps/<n>-<slug>/             never another step; the engine only through its door
      │
      │  may import
      ▼
  bench · toolbox · on-this-device · case-sheet · reopen-case · made-up-spines · open-and-save · sample-films
                                never steps; the engine only through its door
  auto-pass                     never steps, case-state, bench or toolbox;
                                the engine only through its door
  marks · ladder · guide        draw from props: never steps, case-state, view-state,
                                engine-room, bench, auto-pass or toolbox
  film-viewer · spine-3d        never steps, case-state, view-state or engine-room
  review                        of the app's own areas, only shapes, words and
                                look-and-feel/data-attributes
  look-and-feel · choreography · building-blocks
                                of the app's own areas, only each other, words,
                                shapes and gallery/gallery-page

  the engine's door             engine-room/engine-client and engine-room/use-engine
  engine-room/wasm-build/ and engine-room/workers/
                                only from inside engine-room
  engine-room/stand-in/         only from pick-engine.ts, the stand-in itself and tests
  sample-case/                  only from *.gallery.tsx, *.scenes.ts, tests, sample-case
                                itself, the stand-in and pick-engine.ts
  shared types                  only from @/shapes
```

Two more hold by agreement rather than by the lint rule: only `app-shell`
imports `steps/all-steps`, and only `src/main.tsx` imports `gallery/`,
behind `import.meta.env.DEV`, beyond the `GalleryPage` type that gallery
pages take from `gallery/gallery-page`.

Only `src/engine-room/load-engine.ts` may make a network call, and only
to fetch the engine's own `.wasm` from the page's origin.

### Giving a new area its rule

1. Make the folder under `src/`. No folder inside an area takes another
   area's name, because the rule matches an area's folder at any depth.
2. In `eslint.config.js`, add a row to `AREAS`, after every row that
   also matches its files: `{ files: 'app/src/<area>/**/*.{ts,tsx}',
   banned: [...] }`, or add the area to the braces of a row whose list
   is already right. A later row replaces an earlier one for the files
   it matches, so the list is whole: start it with `SHAPES_ONLY` and
   `ENGINE_INSIDES`, use `no('<other-area>')` for each area it never
   imports, and `ENGINE_DOOR_ONLY` if it reaches the engine only
   through its door. The stand-in and sample-case rules join every row
   by themselves.
3. Unless the new area is itself a building block, add its name to
   `ABOVE_THE_BLOCKS`, which keeps `look-and-feel`, `choreography`,
   `building-blocks` and `review` from importing it.
4. Add it to the folder table and the picture above. In the pull
   request, list one import the rule refused and one it allowed.
