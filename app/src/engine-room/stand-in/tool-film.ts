// The film a stand-in tool works on, and what every tool does alike:
// refuse a click it cannot act on, and mark the vertebrae it edited.

import type { ConfidenceReason, FilmFacts, FilmKind, MissReason, Px, VertebraOnFilm } from '@/shapes';
import { EngineFailure } from '../engine-client';

/** The film a tool works on: its kind and size. */
export interface ToolFilm {
  kind: FilmKind;
  facts: FilmFacts;
}

export function markedEdited(vertebra: VertebraOnFilm, dropping: ConfidenceReason[] = []): VertebraOnFilm {
  const reasons = vertebra.confidence.reasons.filter((reason) => !dropping.includes(reason) && reason !== 'edited-by-user');
  return { ...vertebra, confidence: { ...vertebra.confidence, reasons: [...reasons, 'edited-by-user'] } };
}

export function refusal(reason: MissReason): EngineFailure {
  if (reason === 'outside-film') return new EngineFailure({ code: 'tool-click-outside-film', message: 'the click is outside the film', detail: reason });
  return new EngineFailure({ code: 'tool-refused', message: 'the tool cannot act here', detail: reason });
}

export function isBending(kind: FilmKind): boolean {
  return kind === 'bend-left' || kind === 'bend-right';
}

export function outsideFilm(film: ToolFilm, at: Px): boolean {
  return at[0] < -0.5 || at[1] < -0.5 || at[0] > film.facts.width - 0.5 || at[1] > film.facts.height - 0.5;
}
