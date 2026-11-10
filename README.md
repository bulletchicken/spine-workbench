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
