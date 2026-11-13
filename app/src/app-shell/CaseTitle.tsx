import { SHELL_WORDS } from '@/words/shell';
import { isOnDisk, type CaseSaved } from './case-saved';
import './CaseTitle.css';

const WORDS: Record<CaseSaved, string> = {
  'not-saved-yet': SHELL_WORDS.notSavedYet,
  'changes-not-saved': SHELL_WORDS.changesNotSaved,
  'saved-just-now': SHELL_WORDS.savedJustNow,
  'as-saved': SHELL_WORDS.asSaved,
};

interface CaseTitleProps {
  /** The case's name, or `null` for a case that has none yet. */
  name: string | null;
  saved: CaseSaved;
}

/**
 * The case's name and whether it is saved, under the app's name. The
 * mark beside the words is a ring until the case on screen is the case on
 * disk, and a solid dot while it is.
 */
export function CaseTitle({ name, saved }: CaseTitleProps) {
  return (
    <span className="case-title">
      <span className="case-title-name">{name ?? SHELL_WORDS.untitledCase}</span>
      <span className="case-title-state" data-saved={isOnDisk(saved) || undefined}>
        <i aria-hidden="true" />
        {WORDS[saved]}
      </span>
    </span>
  );
}
