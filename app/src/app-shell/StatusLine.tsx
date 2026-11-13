import { faultWords } from '@/words/faults';
import { PREVIEW_NOTICE } from '@/words/preview-notice';
import { SHELL_WORDS } from '@/words/shell';
import type { Position } from './status-readings';
import './StatusLine.css';
import type { EngineStatus } from './use-engine-status';

interface StatusLineProps {
  engine: EngineStatus;
  /** Where the pointer is on a film, or `null` when it is on none. */
  position: Position | null;
  /** The pixel size to show, as written, or `null` for none. */
  pixelSize: string | null;
  /** Requests sent to anywhere but the app's own origin. */
  sent: number;
}

/**
 * The line across the foot of the window: the standing claim on the left,
 * word for word the approved notice; on the right, where the pointer is
 * on a film, that film's pixel size, and how many requests have been
 * sent, which is the same count the promise shows.
 *
 * It also carries the engine's state and version for the browser's
 * guards to read: `data-status` on the line itself, and the version in a
 * span that is spoken and not shown. There is no status light. An engine
 * that did not start is said in words, here, for as long as that is so.
 */
export function StatusLine({ engine, position, pixelSize, sent }: StatusLineProps) {
  return (
    <footer className="status-line" aria-label={SHELL_WORDS.statusLabel} data-testid="engine-status" data-status={engine.status}>
      <span className="status-line-claim">{PREVIEW_NOTICE.short}</span>
      <span className="status-line-readings">
        {engine.status === 'failed' && <span className="status-line-fault">{faultWords(engine.error.code).title}</span>}
        {position !== null && <span>{SHELL_WORDS.position(position.x, position.y, position.unit)}</span>}
        {pixelSize !== null && <span>{SHELL_WORDS.pixelSize(pixelSize)}</span>}
        <span className="status-line-sent">
          {SHELL_WORDS.requestsSent} <b>{sent}</b>
        </span>
        {engine.status === 'ready' && (
          <span className="status-line-spoken">
            {SHELL_WORDS.engineVersion} <span data-testid="engine-version">{engine.info.engineVersion}</span>
          </span>
        )}
      </span>
    </footer>
  );
}
