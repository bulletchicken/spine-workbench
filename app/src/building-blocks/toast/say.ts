// say(): how any part of the app tells the person, in one line, what just
// happened. The line shows as a toast at the foot of the bench (Toasts)
// and is spoken as a status message. One line at a time: a newer line
// takes the place of the one on show.

/** The one thing a toast can offer to do, such as Undo. */
export interface ToastAction {
  label: string;
  /** The keys that do the same, shown as a hint: `['Mod', 'z']`. */
  keys?: readonly string[];
  press: () => void;
}

/** `done`: something changed, and the line says what. `note`: nothing changed, or something to know. */
export type ToastTone = 'done' | 'note';

export interface Said {
  /** Counts up, so the same line said twice is two toasts. */
  id: number;
  line: string;
  tone: ToastTone;
  action: ToastAction | null;
}

let onShow: Said | null = null;
let count = 0;
const listeners = new Set<() => void>();

function tellListeners(): void {
  for (const listener of listeners) listener();
}

/** Say one line. It stays for `--t-toast-stay`, or until a newer line or a press of its action. */
export function say(line: string, options: { tone?: ToastTone; action?: ToastAction } = {}): void {
  count += 1;
  onShow = { id: count, line, tone: options.tone ?? 'done', action: options.action ?? null };
  tellListeners();
}

/** Put the line with this id away, if it is still the one on show. */
export function putAway(id: number): void {
  if (onShow?.id !== id) return;
  onShow = null;
  tellListeners();
}

/** The line on show, or none. */
export function saidNow(): Said | null {
  return onShow;
}

/** Call `listener` whenever the line on show changes. Returns the way to stop. */
export function onSaid(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}
