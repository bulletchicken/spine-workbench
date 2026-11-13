import type { ComponentType, ReactNode } from 'react';
import { LitStandsDown } from '@/guide/lit-stands-down';
import { SHELL_WORDS } from '@/words/shell';
import './GuideColumn.css';

interface GuideColumnProps {
  /** The step on screen, or `null` when there are no steps. */
  step: { slug: string; Screen: ComponentType } | null;
  /** The opening screen, when the app has one: it stands round the step's Screen and shows its own guide while a case is opened or refused. */
  OpeningCase: ComponentType<{ children: ReactNode }> | null;
  /** A sheet with a lit button of its own is open, so the guide's lit button stands down to an outline. */
  sheetIsOpen: boolean;
}

/**
 * The left column: the guide of the step on screen. It scrolls by itself
 * when a state's cards run past the foot of the window, and nothing else
 * in the frame moves.
 */
export function GuideColumn({ step, OpeningCase, sheetIsOpen }: GuideColumnProps) {
  // Keyed by the step, so one step's Screen never keeps another's state.
  const screen = step === null ? null : <step.Screen key={step.slug} />;
  return (
    <aside className="guide-column" aria-label={SHELL_WORDS.guideLabel}>
      <LitStandsDown value={sheetIsOpen}>{OpeningCase === null ? screen : <OpeningCase>{screen}</OpeningCase>}</LitStandsDown>
    </aside>
  );
}
