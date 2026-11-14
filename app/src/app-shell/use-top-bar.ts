import { useState } from 'react';
import { hasCaseToSave, saveCurrentCase } from '@/bench/save-case';
import { useEngine } from '@/engine-room/use-engine';
import { useStore } from 'zustand';
import { threeState } from '@/view-state/three-state';
import { spineBuild } from '@/view-state/spine-build';
import { useCase } from '@/case-state/case-store';
import { popoverOpen, usePopoverOpen } from '@/on-this-device/popover-open';
import { useRequestsSent } from '@/on-this-device/requests-sent';
import type { StepContract } from '@/steps/step-contract';
import { currentStep, useCurrentStep } from '@/view-state/current-step';
import { useFilmIntakeState } from '@/view-state/film-intake';
import { railSteps } from './rail-steps';
import type { TopBarProps } from './TopBar';

/** The bar's actions and state, read from the current visit. */
export function useTopBar(steps: readonly StepContract[]): TopBarProps {
  const engine = useEngine();
  const [saving, setSaving] = useState(false);
  useStore(spineBuild);
  useStore(threeState, (s) => s.moving);
  const { slug } = useCurrentStep();
  const c = useCase((store) => store.case);
  const promiseOpen = usePopoverOpen().open;
  const requests = useRequestsSent();
  const intake = useFilmIntakeState();
  return {
    rail: railSteps(steps, c, slug, intake.reading.length > 0 || intake.making),
    onGoToStep: (to) => { currentStep.getState().go(to); },
    ...(hasCaseToSave(c) ? { onSave: () => {
      if (saving) return;
      setSaving(true);
      void saveCurrentCase(engine).finally(() => { setSaving(false); });
    } } : {}),
    saving,
    requests,
    promiseOpen,
    onPromiseOpenChange: (open) => { popoverOpen.getState().setOpen(open); },
  };
}
