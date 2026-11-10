// What every tool you pick up provides, and what it is given. FROZEN:
// changes only through a contract-change pull request.
//
// A tool is a small state machine, not a component. The film viewer turns
// pointer events into film pixels and hands them to the active tool; the
// tool asks the engine, previews what a click would do before any click,
// and changes the case only through case edits. After anything that
// changes what it shows, the tool calls `changed()`, and the viewer reads
// `view()` again and draws the preview as a layer over the film.
//
// Rules every tool keeps:
// - Nothing happens on a click that the hover did not show.
// - Every engine request carries the session's case version, and every
//   answer is applied as an edit `basedOn` that version, so the case
//   store drops an answer that arrives after its film changed, or after
//   the case gave out the vertebra id it uses.
// - The right button belongs to the viewer (brightness and contrast); a
//   tool sees the primary button only.

import type { ReactNode } from 'react';
import type { FilmGeometry, FilmKind, FilmMarks, Px } from '@/shapes';
import type { CaseEdit } from '@/case-state/case-edits';
import type { EngineClient } from '@/engine-room/engine-client';
import type { FilmSpace, ScreenPoint } from '@/film-viewer/film-space';

export type ToolId = 'region-wand' | 'boundary-wand' | 'pin' | 'ruler' | 'known-object';

/** One pointer event, already in film pixels. */
export interface ToolPointer {
  /** Original film pixels. */
  at: Px;
  screen: ScreenPoint;
  /** The point is on the film, not on the margin around it. */
  onFilm: boolean;
  /** Alt (Option) held: the pin tool unpins. */
  alt: boolean;
  shift: boolean;
  device: 'mouse' | 'pen' | 'touch';
}

/** The film a tool works on, as it is right now. */
export interface ToolSession {
  film: FilmKind;
  /** The case store's version, which requests carry; not `CASE_VERSION`. */
  caseVersion: number;
  /** `null` until the film has marks. */
  marks: FilmMarks | null;
  geometry: FilmGeometry;
  /** The id a vertebra created now would get. */
  nextVertebraId: number;
  engine: EngineClient;
  /** Change the case. */
  apply(edit: CaseEdit): void;
}

/** What the viewer draws for the active tool. */
export interface ToolView {
  /** A CSS cursor value. */
  cursor: string;
  /** One short line about what a click would do here, from the tool's words; `null` for none. */
  hint: string | null;
  /**
   * The preview: what a click would add, remove, split or merge, or the
   * drag in progress. Drawn like a FilmLayer, in film pixels.
   */
  preview: ((space: FilmSpace) => ReactNode) | null;
}

/** A tool while it is picked up. */
export interface ActiveTool {
  pointerDown(pointer: ToolPointer): void;
  pointerMove(pointer: ToolPointer): void;
  pointerUp(pointer: ToolPointer): void;
  /** The pointer left the film viewer. */
  pointerLeave(): void;
  /** Esc: drop the preview or the drag in progress, change nothing. */
  escape(): void;
  view(): ToolView;
  /** Another tool was picked, or the film went away: cancel what is in flight. */
  stop(): void;
}

export interface ToolContract {
  id: ToolId;
  /** The key that picks the tool up: one lower-case letter. */
  key: string;
  /** Its name in the tool dock, from the tool's words. */
  label: string;
  /**
   * Pick the tool up. `session` always returns the film as it is now;
   * `changed` asks the viewer to read `view()` again.
   */
  start(session: () => ToolSession, changed: () => void): ActiveTool;
}
