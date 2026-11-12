import { useCase } from '@/case-state/case-store';
import { popoverOpen, usePopoverOpen } from '@/on-this-device/popover-open';
import { useRequestsSent } from '@/on-this-device/requests-sent';
import type { StepContract } from '@/steps/step-contract';
import { currentStep, useCurrentStep } from '@/view-state/current-step';
import { useFilmIntakeState } from '@/view-state/film-intake';
import { railSteps } from './rail-steps';
import type { TopBarProps } from './TopBar';

/** The bar's five props, read from the current visit. */
export function useTopBar(steps: readonly StepContract[]): TopBarProps {
  const { slug } = useCurrentStep();
  const c = useCase((store) => store.case);
  const promiseOpen = usePopoverOpen().open;
  const requests = useRequestsSent();
  const intake = useFilmIntakeState();
  return {
    rail: railSteps(steps, c, slug, intake.reading.length > 0 || intake.making),
    onGoToStep: (to) => { currentStep.getState().go(to); },
    requests,
    promiseOpen,
    onPromiseOpenChange: (open) => { popoverOpen.getState().setOpen(open); },
  };
}
