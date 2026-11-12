import { Segmented } from '@/building-blocks/Segmented';
import { SHELL_WORDS } from '@/words/shell';
import type { RailStep } from './rail-steps';

/** Reached steps are ways back; the third moment waits for its step. */
export function StepSwitch({ rail, onGo }: { rail: readonly RailStep[]; onGo: (slug: string) => void }) {
  const find = rail.findIndex((step) => step.slug === 'find');
  const hasLaterStep = find >= 0 && find < rail.length - 1;
  return (
    <Segmented
      className="steps"
      label={SHELL_WORDS.stepsLabel}
      options={[
        ...rail.map((step) => ({ id: step.slug, label: step.title, disabled: step.state !== 'now' && !step.open })),
        ...(hasLaterStep ? [] : [{ id: 'three', label: SHELL_WORDS.stepThree, disabled: true }]),
      ]}
      chosen={rail.find((step) => step.state === 'now')?.slug ?? null}
      onChoose={onGo}
    />
  );
}
