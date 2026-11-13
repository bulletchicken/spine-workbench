import { memo } from 'react';
import { useStore } from 'zustand';
import { caseStore, useCase } from '@/case-state/case-store';
import { motionMode } from '@/choreography/reduced-motion';
import { useSystemReducedMotion } from '@/choreography/use-system-reduced-motion';
import type { ToolAttr } from '@/look-and-feel/data-attributes';
import { useRequestsSent } from '@/on-this-device/requests-sent';
import { allSteps } from '@/steps/all-steps';
import type { StepContract } from '@/steps/step-contract';
import type { ToolId } from '@/toolbox/tool-contract';
import { activeTool } from '@/view-state/active-tool';
import { useCaseSheetOpen } from '@/view-state/case-sheet-open';
import { currentStep, useCurrentStep } from '@/view-state/current-step';
import { usePointer } from '@/view-state/pointer';
import { usePreferences } from '@/view-state/preferences';
import { SHELL_WORDS } from '@/words/shell';
import './AppFrame.css';
import { useGlobalKeys } from './global-keys';
import { GuideColumn } from './GuideColumn';
import { stepOnScreen } from './rail-steps';
import { findRoomParts, type RoomParts } from './room-parts';
import { pixelSizeShown, positionOf } from './status-readings';
import { StatusLine } from './StatusLine';
import { stepAttribute } from './step-attribute';
import { TopBar } from './TopBar';
import { useEngineStatus, type EngineStatus } from './use-engine-status';
import { useTopBar } from './use-top-bar';
import { saveIsLive, undoIsLive } from './what-is-live';

/** `data-tool` for the three adjusting tools. The ruler and no tool at all leave the attribute off. */
const TOOL_ATTRIBUTES: Partial<Record<ToolId, ToolAttr>> = { pin: 'pins', 'region-wand': 'region', 'boundary-wand': 'boundary' };

const FOUND_STEPS = allSteps();
const FOUND_PARTS = findRoomParts();

/** The top bar, drawn from the stores. On its own, so an edit to the case draws the bar again and not the room. */
function LiveTopBar({ steps }: { steps: readonly StepContract[] }) {
  return <TopBar {...useTopBar(steps)} />;
}

/** The status line, drawn from the stores. On its own, so the pointer moving over a film draws this line again and nothing else. */
function LiveStatusLine({ engine }: { engine: EngineStatus }) {
  const reading = usePointer().reading;
  const c = useCase((store) => store.case);
  const { sent } = useRequestsSent();
  return <StatusLine engine={engine} position={positionOf(reading, c)} pixelSize={pixelSizeShown(c, reading?.film ?? null)} sent={sent} />;
}

/**
 * The bench region: the bench, mounted once for the whole visit, and the
 * case sheet over it. Drawn again only if its parts change, which they
 * never do, so a step changing or a tool being picked up leaves it be.
 */
const BenchRegion = memo(function BenchRegion({ Bench, CaseSheet }: Pick<RoomParts, 'Bench' | 'CaseSheet'>) {
  return (
    <section className="app-frame-bench" aria-label={SHELL_WORDS.benchLabel}>
      {Bench !== null && <Bench />}
      {CaseSheet !== null && <CaseSheet />}
    </section>
  );
});

interface AppFrameProps {
  /** The steps, in order. The app's own, found by their folders, unless a test or a gallery page hands in others. */
  steps?: readonly StepContract[];
  /** The bench, the case sheet and the opening screen. The app's own, found by their paths, unless handed in. */
  parts?: RoomParts;
}

/**
 * The frame that never rearranges: three rows, the top bar, the room and
 * the status line, with the room split into the guide column and the
 * bench. Learn it once.
 *
 * The bench is mounted once and stays mounted, so films never unmount
 * between steps; the step on screen renders its guide into the guide
 * column and tells the bench what it wants. The frame's root carries the
 * state every part's CSS may read: `data-step`, `data-motion` and
 * `data-tool`. (`data-lane` is the bench's to write, on this same root.)
 */
export function AppFrame({ steps = FOUND_STEPS, parts = FOUND_PARTS }: AppFrameProps) {
  const { slug } = useCurrentStep();
  const onScreen = stepOnScreen(steps, slug);
  const systemAsksForReduced = useSystemReducedMotion();
  const appAsksForReduced = usePreferences().motion === 'reduced';
  // The tool alone: a drag starting or the magnifier turning on is not the frame's to draw.
  const tool = useStore(activeTool, (state) => state.tool);
  const sheetIsOpen = useCaseSheetOpen().open;
  const engine = useEngineStatus();

  // What is live is asked at each press, of the stores themselves.
  useGlobalKeys({
    undoIsLive: () => undoIsLive(stepOnScreen(steps, currentStep.getState().slug)?.slug ?? null),
    saveIsLive: () => saveIsLive(steps, caseStore.getState().case),
  });

  return (
    <div
      className="app-frame"
      data-step={onScreen === null ? '' : stepAttribute(onScreen.slug)}
      data-motion={motionMode(systemAsksForReduced, appAsksForReduced)}
      data-tool={tool === null ? undefined : TOOL_ATTRIBUTES[tool]}
    >
      <LiveTopBar steps={steps} />
      <main className="app-frame-room">
        <GuideColumn step={onScreen} OpeningCase={parts.OpeningCase} sheetIsOpen={sheetIsOpen} />
        <BenchRegion Bench={parts.Bench} CaseSheet={parts.CaseSheet} />
      </main>
      <LiveStatusLine engine={engine} />
    </div>
  );
}
