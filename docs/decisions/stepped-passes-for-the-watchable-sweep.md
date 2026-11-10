# Stepped passes for the watchable sweep

**Decided:** long engine passes are state machines, not functions with
callbacks. `FilmPass` and `ChainPass` each expose `advance(workUnits)`,
which does a bounded slice of work and returns a `Step { done, stage,
fraction, frames }`. The worker's step loop calls it repeatedly.

## Why

- Rust never calls into JavaScript, and WebAssembly has no clock: the
  engine bans `Instant::now`. The worker times each slice with
  `performance.now()` and adapts the work units so a slice takes about
  8 ms.
- Between slices the worker yields, so a cancel can land and progress
  can be sent, without threads or shared memory.
- Frames come from fixed checkpoints in the algorithm, not from the
  slicing, so the sweep shown in the browser is exactly what a native run
  produces, and the last frame equals a direct run's answer.

## Consequences

- Progress is throttled to ten a second and sweep frames to thirty a
  second, newest first; keyframes are never dropped.
- Pacing belongs to the app: the step 3 frame player gives each stage a
  minimum dwell, offers skip and replay, and calms down under reduced
  motion.
- Cancel is a flag checked between slices; a slice that ever runs past
  two seconds gets the worker replaced.
- One-shot tools (the wands, pins) are not steppers; each must answer
  quickly on its own.
