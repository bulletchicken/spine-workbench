import { Check, Info } from 'lucide-react';
import { Toasts } from '@/building-blocks/toast/Toasts';
import { memo } from 'react';
import { useStore } from 'zustand';
import { motionMode } from '@/choreography/reduced-motion';
import { useSystemReducedMotion } from '@/choreography/use-system-reduced-motion';
import { Dock } from '@/guide/Dock';
import { LitStandsDown } from '@/guide/lit-stands-down';
import type { ToolAttr } from '@/look-and-feel/data-attributes';
import { usePageState } from '@/look-and-feel/page-state';
import { allSteps } from '@/steps/all-steps';
import type { StepContract } from '@/steps/step-contract';
import type { ToolId } from '@/toolbox/tool-contract';
import { activeTool } from '@/view-state/active-tool';
import { useCaseSheetOpen } from '@/view-state/case-sheet-open';
import { useCurrentStep } from '@/view-state/current-step';
import { useDockNow } from '@/view-state/dock-now';
import { usePreferences } from '@/view-state/preferences';
import { faultWords } from '@/words/faults';
import { SHELL_WORDS } from '@/words/shell';
import './AppFrame.css';
import { SpineHost } from './spine-host';
import { PreviewNotice } from './PreviewNotice';
import { stepOnScreen } from './rail-steps';
import { findRoomParts, type RoomParts } from './room-parts';
import { stepAttribute } from './step-attribute';
import { TopBar } from './TopBar';
import { useEngineStatus, type EngineStatus } from './use-engine-status';
import { useTopBar } from './use-top-bar';

const TOOL_ATTRIBUTES: Partial<Record<ToolId, ToolAttr>> = { pin: 'pins', 'region-wand': 'region', 'boundary-wand': 'boundary' };
const FOUND_STEPS = allSteps();
const FOUND_PARTS = findRoomParts();

/** Case edits redraw the bar without redrawing the stage. */
function LiveTopBar({ steps }: { steps: readonly StepContract[] }) {
  return <TopBar {...useTopBar(steps)} />;
}

/** Dock updates stay here, so a step handing up new words does not redraw itself. */
function LiveDock({ engine }: { engine: EngineStatus }) {
  const spec = useDockNow();
  return <Dock spec={engine.status === 'failed' ? {
    scene: 'engine',
    sentence: `${faultWords(engine.error.code).title}.`,
    helper: { kind: 'words', words: [SHELL_WORDS.reload] },
  } : spec} />;
}

/** The same bench stays mounted as the step changes. */
const BenchRegion = memo(function BenchRegion({ Bench, CaseSheet }: Pick<RoomParts, 'Bench' | 'CaseSheet'>) {
  return (
    <main className="stage" aria-label={SHELL_WORDS.stageLabel}>
      {Bench !== null && <Bench />}
      {CaseSheet !== null && <CaseSheet />}
    </main>
  );
});

interface AppFrameProps {
  steps?: readonly StepContract[];
  parts?: RoomParts;
}

/** The one stage, with a fixed bar, dock and notice inside the centred frame. */
export function AppFrame({ steps = FOUND_STEPS, parts = FOUND_PARTS }: AppFrameProps) {
  const { slug } = useCurrentStep();
  const onScreen = stepOnScreen(steps, slug);
  const systemAsksForReduced = useSystemReducedMotion();
  const appAsksForReduced = usePreferences().motion === 'reduced';
  const tool = useStore(activeTool, (state) => state.tool);
  const sheetIsOpen = useCaseSheetOpen().open;
  const engine = useEngineStatus();
  const Screen = onScreen?.Screen;
  const OpeningCase = parts.OpeningCase;
  const screen = Screen === undefined ? null : <Screen key={onScreen?.slug} />;
  usePageState('moment', onScreen?.slug === 'place' ? 'place' : onScreen?.slug === 'three' ? 'three' : onScreen?.slug === 'find' ? 'find' : 'films');

  return (
    <div
      className="app-frame"
      data-step={onScreen === null ? '' : stepAttribute(onScreen.slug)}
      data-motion={motionMode(systemAsksForReduced, appAsksForReduced)}
      data-tool={tool === null ? undefined : TOOL_ATTRIBUTES[tool]}
    >
      <div className="glow" aria-hidden="true" />
      <div className="frame">
        <SpineHost>
        <LiveTopBar steps={steps} />
        <BenchRegion Bench={parts.Bench} CaseSheet={parts.CaseSheet} />
        <LitStandsDown value={sheetIsOpen}>
          {OpeningCase === null ? screen : <OpeningCase>{screen}</OpeningCase>}
          <LiveDock engine={engine} />
        </LitStandsDown>
        <PreviewNotice engine={engine} />
        <Toasts marks={{ done: <Check strokeWidth="var(--icon-stroke)" />, note: <Info strokeWidth="var(--icon-stroke)" /> }} />
        </SpineHost>
      </div>
    </div>
  );
}
