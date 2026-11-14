import { useLayoutEffect } from 'react';
import { useStore } from 'zustand';
import { hasCurrentFit } from '@/bench/three/build-spine';
import { useCase } from '@/case-state/case-store';
import { currentStep } from '@/view-state/current-step';
import { useShowInDock } from '@/view-state/dock-now';
import { spineBuild } from '@/view-state/spine-build';
import { threeState } from '@/view-state/three-state';
import { THREE_WORDS } from '@/words/three';
import { threeDock } from './three-dock';
export function ThreeScreen() {
  const c = useCase((s) => s.case);
  const accepted = useStore(spineBuild, (s) => s.accepted);
  const chosen = useStore(threeState, (s) => s.chosen);
  const valid = hasCurrentFit(c);
  useLayoutEffect(() => {
    if (!valid) currentStep.getState().go(c.marks.some((f) => f.film === 'front' && f.vertebrae.length > 0) ? 'find' : 'films');
  }, [valid, c]);
  useShowInDock(valid && accepted !== null ? threeDock(accepted.fit, accepted.input.side !== null, chosen, (name) => { threeState.getState().choose(name); }) : { scene: 'building', sentence: THREE_WORDS.building });
  return null;
}
