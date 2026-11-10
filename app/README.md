# app

Everything you see and click, in TypeScript and React. The computing
happens in the Rust engine, which runs in background workers; the app
reaches it only through the engine room.

## Folders

| Folder | Holds |
|---|---|
| `src/steps/` | the five steps, numbered in the order a surgeon uses them |
| `src/app-shell/` | the frame around every step |
| `src/film-viewer/` | the film viewer: pixels on the GPU, marks drawn over them |
| `src/toolbox/` | the adjusting tools you pick up |
| `src/made-up-spines/` | "Try a made-up spine" and its answer key |
| `src/engine-room/` | the loader, the workers, their messages and the engine client |
| `src/case-state/` | the open case (what gets saved) and its undo history |
| `src/view-state/` | what you are looking at right now (never saved) |
| `src/open-and-save/` | files in and out, with no network |
| `src/shapes/` | the shared types, generated from the Rust shapes |
| `src/look-and-feel/`, `src/choreography/`, `src/building-blocks/` | the visual language, motion and small UI pieces |
| `src/words/` | every sentence the user reads |
| `build-plugins/` | the content policy and the engine build, as Vite plugins |

## Who may import whom

ESLint enforces these rules.

```
            steps/<n>-<verb>/  ── never another step
                  │
                  │  may import
                  ▼
  film-viewer · toolbox · made-up-spines · case-state · view-state · open-and-save
  look-and-feel · choreography · building-blocks · words · shapes
  engine-room/engine-client · engine-room/use-engine   (and nothing else in engine-room)

  film-viewer · look-and-feel · choreography · building-blocks
        never import steps, case-state or engine-room

  engine-room/wasm-build/ and engine-room/workers/   only from inside engine-room
  engine-room/stand-in/                              only from pick-engine.ts and tests
  shared types                                       only from @/shapes
```

Only `src/engine-room/load-engine.ts` may make a network call, and only
to fetch the engine's own `.wasm` from the page's origin.
