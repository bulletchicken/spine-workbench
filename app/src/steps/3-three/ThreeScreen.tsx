import { useLayoutEffect } from 'react';
import { useStore } from 'zustand';
import { hasCurrentFit } from '@/bench/three/build-spine';
import { useNarrowThree } from '@/bench/three/Measurements';
import { useCase } from '@/case-state/case-store';
import type { DockSpec } from '@/guide/dock-spec';
import { currentStep } from '@/view-state/current-step';
import { useShowInDock } from '@/view-state/dock-now';
import { spineBuild } from '@/view-state/spine-build';
import { threeState } from '@/view-state/three-state';
import { THREE_WORDS } from '@/words/three';
import { READINGS_WORDS } from '@/words/spine-readings';
import { threeDock } from './three-dock';
export function ThreeScreen() {
  const c = useCase((s) => s.case);
  const accepted = useStore(spineBuild, (s) => s.accepted);
  const { compared, chosen, moving, sheet } = useStore(threeState);
  const narrow = useNarrowThree();
  const valid = hasCurrentFit(c);
  useLayoutEffect(() => {
    if (!valid) currentStep.getState().go(c.marks.some((f) => f.film === 'front' && f.vertebrae.length > 0) ? 'find' : 'films');
  }, [valid, c]);
  let dock: DockSpec = { scene: 'building', sentence: THREE_WORDS.building };
  if (valid && accepted !== null) {
    if (compared !== null) dock = {
      scene: 'compare', sentence: THREE_WORDS.comparison(compared),
      helper: { kind: 'words', words: [compared === 'front' || compared === 'side' ? THREE_WORDS.compareHelper : THREE_WORDS.bendHelper] },
      secondary: { label: THREE_WORDS.done, press: () => {
        const origin = document.querySelector<HTMLElement>(`.tile[data-place="${compared}"]`);
        threeState.getState().compare(null); origin?.focus();
      } },
    };
    else {
      const rest = threeDock(accepted.fit, accepted.input.side !== null, chosen, (name) => { if (!threeState.getState().moving && !threeState.getState().sheet) threeState.getState().choose(name); });
      dock = moving ? { scene: 'three', sentence: rest.sentence, ...(rest.helper === undefined ? {} : { helper: rest.helper }) } : {
        ...rest,
        ...(narrow ? { quiet: { label: READINGS_WORDS.title, controls: 'measurements', expanded: sheet, press: () => { threeState.getState().showSheet(!sheet); } } } : {}),
      };
    }
  }
  useShowInDock(dock);
  return null;
}
