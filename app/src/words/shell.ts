// The frame's own words: the top bar, the case's saved state, undo and
// redo, the Save case button and the status line. The standing claim in
// the status line is not here: it is the approved notice in
// preview-notice.ts, used word for word.

import type { ActionRecord } from '@/case-state/action-record';

export const SHELL_WORDS = {
  appName: 'Spine Workbench',

  // What each region of the frame is called when it is spoken.
  stepsLabel: 'Steps',
  guideLabel: 'What to do now',
  benchLabel: 'The bench',
  statusLabel: 'Status',

  /** A step on the rail, as it is spoken: its number, its name, and whether it is done. */
  step: (number: number, title: string, done: boolean): string =>
    done ? `Step ${String(number)}, ${title}, done` : `Step ${String(number)}, ${title}`,

  // The case, under the app's name.
  untitledCase: 'Untitled case',
  notSavedYet: 'Not saved yet',
  changesNotSaved: 'Changes not saved',
  savedJustNow: 'Saved just now',
  /** A case that was opened from a file and has not been changed since. */
  asSaved: 'As saved',

  undo: 'Undo',
  redo: 'Redo',
  nothingToUndo: 'Nothing to undo',
  nothingToRedo: 'Nothing to redo',
  saveCase: 'Save case',

  // The status line.
  /** Where the pointer is on a film, in millimetres, or in pixels while the film has no size. */
  position: (x: number, y: number, unit: 'mm' | 'px'): string => `x ${String(x)} ${unit}  y ${String(y)} ${unit}`,
  pixelSize: (mmPerPx: string): string => `${mmPerPx} mm/px`,
  requestsSent: 'Requests sent:',
  engineVersion: 'Engine version',
} as const;

const A_VERTEBRA = 'a vertebra';

/**
 * What an action was, in a few words that start with a capital: "Split
 * T12". The level is the one the vertebra had just before the action.
 */
export function actionName(record: ActionRecord): string {
  const level = record.aimedAt?.level ?? A_VERTEBRA;
  switch (record.kind) {
    case 'replace-marks':
      return engineAnswerName(record, level);
    case 'move-mark':
      return record.aimedAt === null ? 'Moved a landmark' : `Moved a point on ${level}`;
    case 'pin-mark':
      return record.aimedAt === null ? 'Changed a pin on a landmark' : `Changed a pin on ${level}`;
    case 'hide-pedicle':
      return `Marked a pedicle of ${level} not visible`;
    case 'confirm-l5':
      return 'Named L5';
    case 'confirm-vertebra':
      return `Confirmed ${level}`;
    case 'set-film':
      return 'Hung a film';
    case 'remove-film':
      return 'Took a film down';
    case 'set-geometry':
      return 'Changed a film’s size or facing';
    case 'flip-brightness':
      return 'Flipped a film’s brightness';
    case 'set-blanked':
      return 'Blanked part of a film';
    case 'open-case':
      return 'Opened a case';
  }
}

function engineAnswerName(record: ActionRecord, level: string): string {
  const cause = record.cause;
  if (cause === null) return `Changed ${level}`;
  switch (cause.by) {
    case 'auto-pass':
      return 'The automatic pass';
    case 'region-wand':
      return `Reshaped ${level}`;
    case 'boundary-wand':
      return cause.action === 'split' ? `Split ${level}` : `Merged ${level} with its neighbour`;
    case 'pin':
      return record.aimedAt === null ? 'Pinned a landmark' : `Pinned a point on ${level}`;
    case 'unpin':
      return record.aimedAt === null ? 'Unpinned a landmark' : `Unpinned a point on ${level}`;
  }
}

/** The same name inside a sentence: "split T12". */
function midSentence(name: string): string {
  return name.charAt(0).toLowerCase() + name.slice(1);
}

/** The tooltip of the undo button: "Undo: Split T12", or that there is nothing to undo. */
export function undoTip(next: ActionRecord | null): string {
  return next === null ? SHELL_WORDS.nothingToUndo : `${SHELL_WORDS.undo}: ${actionName(next)}`;
}

/** The tooltip of the redo button. */
export function redoTip(next: ActionRecord | null): string {
  return next === null ? SHELL_WORDS.nothingToRedo : `${SHELL_WORDS.redo}: ${actionName(next)}`;
}

/** What is said after an undo: "Undid: split T12." */
export function undidLine(record: ActionRecord): string {
  return `Undid: ${midSentence(actionName(record))}.`;
}

/** What is said after a redo: "Redid: split T12." */
export function redidLine(record: ActionRecord): string {
  return `Redid: ${midSentence(actionName(record))}.`;
}
