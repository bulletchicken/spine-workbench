// The reducer's rules as properties, over random histories of four films
// with engine answers that arrive late and in any order
// (random-histories.ts). Each step is held to the rules with a record the
// test keeps itself: every case seen, by version, and a plain list of
// what each kept action changed, which mirrors an undo list without
// using it.

import { describe, expect, it } from 'vitest';
import type { CaseFileV1, FilmKind } from '@/shapes';
import { editedFilm } from './case-edits';
import type { CaseState } from './case-reducer';
import { eachRandomHistory, type Happened } from './random-histories';
import { sameJson } from './same-json';
import { FILM_KINDS } from './selectors';
import { filmState } from './undo-history';

const RUNS = 300;

/** What a case holds, without the ids it has given out: the films and their marks. */
const held = (c: CaseFileV1) => ({ films: c.films, marks: c.marks });

/** Hold every step of `RUNS` random histories to `rule`, each history with a ledger of its own. */
function check(rule: (happened: Happened, ledger: Ledger) => void): void {
  eachRandomHistory(RUNS, () => {
    const ledger = new Ledger();
    return (happened) => {
      rule(happened, ledger);
      ledger.follow(happened);
    };
  });
}

/** The test's own account of what happened: each case by version, and each kept action's before and after. */
class Ledger {
  readonly byVersion = new Map<number, CaseFileV1>();
  /** One entry per action that could be undone now, then the ones that could be redone, in order. */
  actions: { before: CaseFileV1; after: CaseFileV1 }[] = [];

  follow(happened: Happened): void {
    const { before, after } = happened;
    this.byVersion.set(before.version, before.case);
    this.byVersion.set(after.version, after.case);
    if (happened.did === 'afresh') this.actions = [];
    if (happened.did !== 'action' || happened.report.record === null) return;
    const depth = before.history.undo.length;
    if (happened.edits.some((edit) => edit.kind === 'open-case')) {
      this.actions = after.history.undo.length === 0 ? [] : [{ before: openedCase(happened), after: after.case }];
    } else if (after.history.undo.length === depth) {
      const joined = this.actions[depth - 1] ?? fail('action for a drag to join');
      this.actions = [...this.actions.slice(0, depth - 1), { before: joined.before, after: after.case }];
    } else {
      this.actions = [...this.actions.slice(0, depth), { before: before.case, after: after.case }];
    }
  }
}

/** The case an action opened, before the edits that followed it in the same action. */
function openedCase(happened: Extract<Happened, { did: 'action' }>): CaseFileV1 {
  const last = happened.edits.findLastIndex((edit) => edit.kind === 'open-case');
  const edit = happened.edits[last];
  if (edit?.kind !== 'open-case') throw new Error('the action opened a case');
  return { ...edit.case, films: [...edit.case.films].sort(byKind), marks: [...edit.case.marks].sort((a, b) => FILM_KINDS.indexOf(a.film) - FILM_KINDS.indexOf(b.film)) };
}

const byKind = (a: { kind: FilmKind }, b: { kind: FilmKind }) => FILM_KINDS.indexOf(a.kind) - FILM_KINDS.indexOf(b.kind);

function idsOf(state: CaseState): number[] {
  return state.case.marks.flatMap((marks) => marks.vertebrae.map((vertebra) => vertebra.id));
}

describe('over random histories of four films', () => {
  it('applying an action and then undoing it restores every film it touched; redoing it brings it back', () => {
    check((happened, ledger) => {
      if (happened.did !== 'undo' && happened.did !== 'redo') return;
      if (happened.record === null) return;
      const depth = happened.before.history.undo.length;
      if (happened.did === 'undo') {
        expect(held(happened.after.case)).toStrictEqual(held(ledger.actions[depth - 1]?.before ?? fail('an action to undo')));
      } else {
        expect(held(happened.after.case)).toStrictEqual(held(ledger.actions[depth]?.after ?? fail('an action to redo')));
      }
    });
  });

  it('an undo then a redo leaves the case as it was', () => {
    eachRandomHistory(RUNS, () => {
      let beforeUndo: CaseFileV1 | null = null;
      return (happened) => {
        if (happened.did === 'redo' && happened.record !== null && beforeUndo !== null) {
          expect(held(happened.after.case)).toStrictEqual(held(beforeUndo));
        }
        beforeUndo = happened.did === 'undo' && happened.record !== null ? happened.before.case : null;
      };
    });
  });

  it('the version only goes up: once per kept edit, once per undo or redo, and never otherwise', () => {
    check((happened) => {
      const { before, after } = happened;
      let raised = 0;
      if (happened.did === 'action') raised = happened.report.outcomes.filter((outcome) => outcome.kept).length;
      else if ((happened.did === 'undo' || happened.did === 'redo') && happened.record !== null) raised = 1;
      expect(after.version).toBe(before.version + raised);
      for (const film of FILM_KINDS) expect(after.changedAt[film]).toBeLessThanOrEqual(after.version);
    });
  });

  it('no vertebra id is ever on two vertebrae, and every id is below the next one given out', () => {
    check((happened) => {
      const { before, after } = happened;
      const ids = idsOf(after);
      expect(new Set(ids).size).toBe(ids.length);
      for (const id of ids) expect(id).toBeLessThan(after.case.nextVertebraId);
      const opened = happened.did === 'action' && happened.edits.some((edit) => edit.kind === 'open-case');
      if (!opened) expect(after.case.nextVertebraId).toBeGreaterThanOrEqual(before.case.nextVertebraId);
    });
  });

  it('an id is never given out again: it stays on the film it was first seen on until another case is opened', () => {
    // Uniqueness in each case is not enough: an undo takes ids away, and a
    // late answer could hand one of them to a new vertebra on another film.
    eachRandomHistory(RUNS, () => {
      const firstSeenOn = new Map<number, FilmKind>();
      return (happened) => {
        if (happened.did === 'action' && happened.edits.some((edit) => edit.kind === 'open-case')) firstSeenOn.clear();
        for (const marks of happened.after.case.marks) {
          for (const vertebra of marks.vertebrae) {
            const seen = firstSeenOn.get(vertebra.id);
            if (seen === undefined) firstSeenOn.set(vertebra.id, marks.film);
            else expect(marks.film, `vertebra id ${String(vertebra.id)}`).toBe(seen);
          }
        }
      };
    });
  });

  it('a stale answer never lands: a kept answer finds its film exactly as it was when it was asked for', () => {
    check((happened, ledger) => {
      if (happened.did !== 'action' || !happened.late) return;
      happened.edits.forEach((edit, index) => {
        if (edit.kind !== 'replace-marks' || happened.report.outcomes[index]?.kept !== true) return;
        const asked = ledger.byVersion.get(edit.basedOn) ?? fail('the case the answer was asked from');
        expect(filmState(happened.before.case, edit.marks.film)).toStrictEqual(filmState(asked, edit.marks.film));
      });
    });
  });

  it('each report says truly what landed: a kept answer is in the case, a dropped edit changed nothing', () => {
    check((happened) => {
      if (happened.did !== 'action') return;
      const films = happened.edits.map(editedFilm);
      if (films.includes(null) || new Set(films).size !== films.length) return;
      happened.edits.forEach((edit, index) => {
        const film = editedFilm(edit) ?? fail('film for an edit that touches one');
        if (happened.report.outcomes[index]?.kept !== true) {
          expect(filmState(happened.after.case, film)).toStrictEqual(filmState(happened.before.case, film));
        } else if (edit.kind === 'replace-marks') {
          expect(filmState(happened.after.case, film).marks).toStrictEqual(edit.marks);
        }
      });
    });
  });

  it('an action that keeps nothing leaves no trace', () => {
    check((happened) => {
      if (happened.did !== 'action' || happened.report.outcomes.some((outcome) => outcome.kept)) return;
      expect(happened.report.record).toBeNull();
      expect(happened.after).toBe(happened.before);
    });
  });

  it('a kept region wand answer makes the clicks it was asked with the session, even right after a refused one', () => {
    let kept = 0;
    let afterRefusal = 0;
    eachRandomHistory(RUNS, () => {
      /** Each refused request still at its film's version, as vertebra@version. */
      const refused = new Set<string>();
      return (happened) => {
        if (happened.did === 'unanswered') {
          for (const [id, waiting] of Object.entries(happened.before.wand.sent)) {
            const left = happened.after.wand.sent[Number(id)] ?? [];
            for (const request of waiting) if (!left.includes(request)) refused.add(`${id}@${String(request.askedAt)}`);
          }
        }
        if (happened.did !== 'action' || happened.wandClicks === null) return;
        const [edit] = happened.edits;
        if (edit?.kind !== 'replace-marks' || edit.cause.by !== 'region-wand' || happened.report.outcomes[0]?.kept !== true) return;
        kept += 1;
        if (refused.has(`${String(edit.cause.vertebra)}@${String(edit.basedOn)}`)) afterRefusal += 1;
        expect(happened.after.wand.clicks[edit.cause.vertebra]).toStrictEqual(happened.wandClicks);
      };
    });
    // About 65 kept answers and 15 retries landing right after a refusal per 300 histories.
    expect(kept).toBeGreaterThan(20);
    expect(afterRefusal).toBeGreaterThan(0);
  });

  it("a vertebra keeps its wand clicks only while nothing but its own wand changes it", () => {
    check((happened) => {
      const { before, after } = happened;
      const ownWand = new Set<number>();
      if (happened.did === 'action') {
        happened.edits.forEach((edit, index) => {
          if (edit.kind === 'replace-marks' && edit.cause.by === 'region-wand' && happened.report.outcomes[index]?.kept === true) ownWand.add(edit.cause.vertebra);
        });
      }
      const marksBefore = vertebraMarks(before.case);
      const marksAfter = vertebraMarks(after.case);
      for (const [id, clicks] of Object.entries(after.wand.clicks)) {
        const vertebra = Number(id);
        if (ownWand.has(vertebra)) continue;
        expect(clicks).toStrictEqual(before.wand.clicks[vertebra]);
        expect(sameJson(marksBefore.get(vertebra), marksAfter.get(vertebra))).toBe(true);
      }
    });
  });
});

function vertebraMarks(c: CaseFileV1): Map<number, unknown> {
  return new Map(c.marks.flatMap((marks) => marks.vertebrae.map((vertebra) => [vertebra.id, vertebra.marks] as const)));
}

function fail(what: string): never {
  throw new Error(`the ledger has no ${what}`);
}
