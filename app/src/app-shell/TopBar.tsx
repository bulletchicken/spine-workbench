import { Button } from '@/building-blocks/Button';
import { Tooltip } from '@/building-blocks/Tooltip';
import type { ActionRecord } from '@/case-state/action-record';
import { BrandMark } from '@/look-and-feel/BrandMark';
import { Icon } from '@/look-and-feel/icons/Icon';
import { PromiseBadge } from '@/on-this-device/PromiseBadge';
import type { RequestsSent } from '@/on-this-device/requests-sent';
import { SHELL_WORDS } from '@/words/shell';
import type { CaseSaved } from './case-saved';
import { CaseTitle } from './CaseTitle';
import type { RailStep } from './rail-steps';
import { StepRail } from './StepRail';
import './TopBar.css';
import { UndoRedo } from './UndoRedo';

export interface TopBarProps {
  caseName: string | null;
  saved: CaseSaved;
  rail: readonly RailStep[];
  onGoToStep: (slug: string) => void;
  requests: RequestsSent;
  promiseOpen: boolean;
  onPromiseOpenChange: (open: boolean) => void;
  undoIsLive: boolean;
  nextUndo: ActionRecord | null;
  nextRedo: ActionRecord | null;
  onUndo: () => void;
  onRedo: () => void;
  saveIsLive: boolean;
  onSave: () => void;
}

/**
 * The bar across the top of the window. Over the guide: the mark, the
 * app's name, and under the name the case and whether it is saved. Over
 * the bench: the step rail, then the on-device badge, undo and redo, and
 * Save case, which is outlined like every control that is not the next
 * action.
 */
export function TopBar(props: TopBarProps) {
  return (
    <header className="top-bar">
      <div className="top-bar-brand">
        <BrandMark />
        <span className="top-bar-name">{SHELL_WORDS.appName}</span>
        <CaseTitle name={props.caseName} saved={props.saved} />
      </div>

      <StepRail steps={props.rail} onGo={props.onGoToStep} />

      <div className="top-bar-actions" data-crowded={props.rail.length > 4 || undefined}>
        <PromiseBadge requests={props.requests} open={props.promiseOpen} onOpenChange={props.onPromiseOpenChange} />
        <span className="top-bar-rule" aria-hidden="true" />
        <UndoRedo live={props.undoIsLive} nextUndo={props.nextUndo} nextRedo={props.nextRedo} onUndo={props.onUndo} onRedo={props.onRedo} />
        <Tooltip tip={SHELL_WORDS.saveCase} align="end" spoken={false}>
          <Button className="top-bar-save" disabled={!props.saveIsLive} onPress={props.onSave} aria-label={SHELL_WORDS.saveCase}>
            <Icon name="case-file" />
            <span className="top-bar-save-words">{SHELL_WORDS.saveCase}</span>
          </Button>
        </Tooltip>
      </div>
    </header>
  );
}
