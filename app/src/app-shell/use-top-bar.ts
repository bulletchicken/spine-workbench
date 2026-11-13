// The top bar's state, read from the app's stores: the case and its undo
// list, whether it is saved, the step on screen, the promise's popover and
// its counter. The top bar itself only draws what it is handed.

import { useCase } from '@/case-state/case-store';
import { nextRedo, nextUndo } from '@/case-state/undo-history';
import { popoverOpen, usePopoverOpen } from '@/on-this-device/popover-open';
import { useRequestsSent } from '@/on-this-device/requests-sent';
import type { StepContract } from '@/steps/step-contract';
import { currentStep, useCurrentStep } from '@/view-state/current-step';
import { useSavedState } from '@/view-state/saved-state';
import { caseSaved } from './case-saved';
import { openCaseSheet, redoOnce, undoOnce } from './frame-actions';
import { railSteps, stepOnScreen } from './rail-steps';
import type { TopBarProps } from './TopBar';
import { saveIsLive, undoIsLive } from './what-is-live';

/** What the top bar shows and does for these steps, as the stores stand. */
export function useTopBar(steps: readonly StepContract[]): TopBarProps {
  const { slug } = useCurrentStep();
  const c = useCase((store) => store.case);
  const version = useCase((store) => store.version);
  const history = useCase((store) => store.history);
  const { name, mark } = useSavedState();
  const promiseOpen = usePopoverOpen().open;
  const requests = useRequestsSent();
  const onScreen = stepOnScreen(steps, slug);

  return {
    caseName: name,
    saved: caseSaved(mark, version, c.films.length > 0),
    rail: railSteps(steps, c, slug),
    onGoToStep: (to) => {
      currentStep.getState().go(to);
    },
    requests,
    promiseOpen,
    onPromiseOpenChange: (open) => {
      popoverOpen.getState().setOpen(open);
    },
    undoIsLive: undoIsLive(onScreen?.slug ?? null),
    nextUndo: nextUndo(history),
    nextRedo: nextRedo(history),
    onUndo: undoOnce,
    onRedo: redoOnce,
    saveIsLive: saveIsLive(steps, c),
    onSave: openCaseSheet,
  };
}
