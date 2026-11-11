// The region wand's clicks on each vertebra: the session the engine is
// handed with every new click (`RegionWandInput.clicks`). The store keeps
// them, so nothing in the toolbox remembers clicks, and clears a
// vertebra's clicks whenever anything but its own wand clicks changes it.
//
// The rules for a session are the correction-tools spec's (R2): a
// vertebra's clicks since its corners were last changed by another tool,
// an undo or a redo; at most 20, the oldest dropped first; and a new click
// close to an older click of the other polarity takes its place, since
// only a click in nearly the same place is a change of mind.
//
// A person may click again before the last answer is in, so the clicks
// sent wait in a list per vertebra, each with a number of its own and
// the case version it was asked at. The engine answers a film's region
// wand jobs one at a time, in the order they were asked, and never
// replaces one, so a vertebra's requests end oldest first. A request
// ends one of three ways:
// - Its answer is applied and kept: the vertebra's marks change, so its
//   clicks become the session and every request still waiting for that
//   vertebra goes, since their answers can no longer land.
// - Its answer is applied and dropped: its film changed after it was
//   asked, so every later answer for the vertebra is asked at a version
//   above it and it is never matched again. It goes when the next
//   request is added.
// - It gets no answer: the engine refused it (no contrast, no
//   convergence, a crash) or the tool gave it up. The tool must then say
//   so, by its number, before it applies any later answer; otherwise the
//   next answer at the same version would be taken for its answer. For
//   the same reason a given-up request's answer, if it still arrives
//   (region wand jobs cannot be cancelled), must never be applied: it
//   would be taken for the answer to a retry asked at the same version.
// So a landing answer is matched to the oldest request still waiting at
// the version the answer carries.

import type { CaseFileV1, VertebraMarks, WandClick } from '@/shapes';
import { cornersOf } from './c7-of';
import { sameJson } from './same-json';

/** A session holds at most this many clicks. */
export const MOST_WAND_CLICKS = 20;

/** A new click within this many vertebra heights of an older click of the other polarity replaces it. */
export const CHANGE_OF_MIND = 0.15;

/** Clicks handed to the engine for one vertebra, waiting for the answer. */
export interface SentClicks {
  /** The request's own number, which the tool gives back if it ends with no answer. */
  ask: number;
  /** The case version the request was asked at, which its answer carries as `basedOn`. */
  askedAt: number;
  clicks: readonly WandClick[];
}

export interface WandSessions {
  /** Each vertebra's clicks, oldest first, by vertebra id. */
  clicks: Readonly<Record<number, readonly WandClick[]>>;
  /** The requests waiting for an answer, oldest first, by vertebra id. */
  sent: Readonly<Record<number, readonly SentClicks[]>>;
  /** How many requests have been asked; the next one's number. Never goes down, so a number is never given twice. */
  asks: number;
}

export const NO_WAND_SESSIONS: WandSessions = { clicks: {}, sent: {}, asks: 0 };

/** No clicks and nothing waiting, with request numbers still counting up from where they were. */
export function clearedSessions(sessions: WandSessions): WandSessions {
  return { ...NO_WAND_SESSIONS, asks: sessions.asks };
}

/**
 * A vertebra's height: the mean length of its two side edges, upper to
 * lower corner on each side.
 */
export function vertebraHeight(marks: VertebraMarks): number {
  const [upperA, upperB, lowerB, lowerA] = cornersOf(marks);
  const length = (from: [number, number], to: [number, number]) => Math.hypot(to[0] - from[0], to[1] - from[1]);
  return (length(upperA.at, lowerA.at) + length(upperB.at, lowerB.at)) / 2;
}

/** The session with `click` added, for a vertebra `height` pixels tall. */
export function withClick(session: readonly WandClick[], click: WandClick, height: number): WandClick[] {
  const reach = CHANGE_OF_MIND * height;
  const kept = session.filter(
    (older) => older.polarity === click.polarity || Math.hypot(older.at[0] - click.at[0], older.at[1] - click.at[1]) > reach,
  );
  return [...kept, click].slice(-MOST_WAND_CLICKS);
}

/**
 * The sessions with a request for `vertebra`, asked at `askedAt` with
 * `clicks`, added last, and that request's number. Requests asked before
 * `stillFrom`, the version at which the vertebra's film last changed, are
 * dropped: their answers can no longer land.
 */
export function withSent(
  sessions: WandSessions,
  vertebra: number,
  sent: { askedAt: number; clicks: readonly WandClick[] },
  stillFrom: number,
): { sessions: WandSessions; ask: number } {
  const ask = sessions.asks;
  const waiting = (sessions.sent[vertebra] ?? []).filter((older) => older.askedAt >= stillFrom);
  return { sessions: { ...sessions, sent: { ...sessions.sent, [vertebra]: [...waiting, { ask, ...sent }] }, asks: ask + 1 }, ask };
}

/** The sessions without request `ask`, which ended with no answer; the very same sessions when no such request waits. */
export function withoutSent(sessions: WandSessions, ask: number): WandSessions {
  const waiting = Object.values(sessions.sent).flat();
  if (!waiting.some((request) => request.ask === ask)) return sessions;
  const sent: Record<number, readonly SentClicks[]> = {};
  for (const [id, requests] of Object.entries(sessions.sent)) {
    const left = requests.filter((request) => request.ask !== ask);
    if (left.length > 0) sent[Number(id)] = left;
  }
  return { ...sessions, sent };
}

/** The clicks of the request an answer for `vertebra`, asked at `askedAt`, is the answer to: the oldest still waiting at that version; `null` when none waits. */
export function answeredClicks(sessions: WandSessions, vertebra: number, askedAt: number): readonly WandClick[] | null {
  return sessions.sent[vertebra]?.find((request) => request.askedAt === askedAt)?.clicks ?? null;
}

/**
 * The sessions after the case went from `before` to `after`. Every
 * vertebra whose marks changed, or that is gone, loses its clicks and its
 * waiting requests, except that `landed`, the vertebra whose own region
 * wand answer this was, takes as its session the clicks that answer was
 * asked with.
 */
export function afterChange(
  sessions: WandSessions,
  before: CaseFileV1,
  after: CaseFileV1,
  landed: { vertebra: number; clicks: readonly WandClick[] | null } | null,
): WandSessions {
  const was = marksById(before);
  const now = marksById(after);
  const unchanged = (id: number) => id !== landed?.vertebra && now.has(id) && sameJson(was.get(id), now.get(id));
  const clicks = keep(sessions.clicks, unchanged);
  const sent = keep(sessions.sent, unchanged);
  if (landed !== null && landed.clicks !== null) clicks[landed.vertebra] = landed.clicks;
  return sameJson(clicks, sessions.clicks) && sameJson(sent, sessions.sent) ? sessions : { clicks, sent, asks: sessions.asks };
}

function keep<T>(byId: Readonly<Record<number, T>>, unchanged: (id: number) => boolean): Record<number, T> {
  const kept: Record<number, T> = {};
  for (const [id, value] of Object.entries(byId)) if (unchanged(Number(id))) kept[Number(id)] = value;
  return kept;
}

function marksById(c: CaseFileV1): Map<number, VertebraMarks> {
  const byId = new Map<number, VertebraMarks>();
  for (const marks of c.marks) for (const vertebra of marks.vertebrae) byId.set(vertebra.id, vertebra.marks);
  return byId;
}
