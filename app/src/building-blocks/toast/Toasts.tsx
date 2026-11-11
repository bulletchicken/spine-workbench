import { useState, useSyncExternalStore, type ReactNode } from 'react';
import { onSaid, putAway, saidNow, type ToastTone } from './say';
import { Toast } from './Toast';

interface ToastsProps {
  /**
   * The icon a line opens with, for each tone: `done` for a line that
   * says what changed (a tick), `note` for one that says nothing did.
   * Handed in by whoever mounts the toasts, as every icon is.
   */
  marks: Readonly<Record<ToastTone, ReactNode>>;
}

/**
 * Where toasts show: the foot of its nearest positioned ancestor. Mounted
 * once, by the bench. It shows whatever `say()` was last given, speaks it
 * as a status message, and puts it away when it has stayed its time or
 * its action is pressed.
 */
export function Toasts({ marks }: ToastsProps) {
  const said = useSyncExternalStore(onSaid, saidNow);
  // The line last shown is kept, so it can be drawn while it fades.
  const [drawn, setDrawn] = useState(said);
  if (said !== null && said !== drawn) setDrawn(said);

  return (
    <div className="toasts">
      <p className="toasts-spoken" role="status">
        {said !== null && <span key={said.id}>{said.line}</span>}
      </p>
      <Toast
        said={drawn}
        shown={said !== null}
        mark={drawn === null ? null : marks[drawn.tone]}
        onAction={() => {
          if (said === null) return;
          putAway(said.id);
          said.action?.press();
        }}
        onStayed={() => {
          if (said !== null) putAway(said.id);
        }}
      />
    </div>
  );
}
