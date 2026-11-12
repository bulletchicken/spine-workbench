import { PREVIEW_NOTICE } from '@/words/preview-notice';
import { SHELL_WORDS } from '@/words/shell';
import type { EngineStatus } from './use-engine-status';

/** The standing notice also carries the engine's state for the browser's checks. */
export function PreviewNotice({ engine }: { engine: EngineStatus }) {
  return (
    <p className="notice" data-testid="engine-status" data-status={engine.status}>
      {PREVIEW_NOTICE.short}
      {engine.status === 'ready' && (
        <span className="spoken-only">
          {SHELL_WORDS.engineVersion} <span data-testid="engine-version">{engine.info.engineVersion}</span>
        </span>
      )}
    </p>
  );
}
