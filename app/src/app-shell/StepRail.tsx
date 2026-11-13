import { Fragment, useLayoutEffect, useRef } from 'react';
import { Tooltip } from '@/building-blocks/Tooltip';
import { Icon } from '@/look-and-feel/icons/Icon';
import { SHELL_WORDS } from '@/words/shell';
import type { RailStep } from './rail-steps';
import './StepRail.css';

/** With more steps than this, the done ones fold down to their tick so the rail still fits. */
const ROOMY = 4;

/**
 * Marks the rail `data-tight` while its steps do not fit the room the top
 * bar gives it, so that only the step on screen keeps its name. The rail
 * is measured with every name shown and folded in the same breath if it
 * runs over, before anything is painted; it is measured again whenever
 * the bar's room, the steps or the typeface's widths change.
 */
function useTightRail(steps: readonly RailStep[]) {
  const rail = useRef<HTMLElement>(null);
  const drawn = steps.map((step) => `${step.title}:${step.state}`).join('|');
  useLayoutEffect(() => {
    const element = rail.current;
    const list = element?.firstElementChild;
    // A page with no layout to measure, as in a unit test, leaves the rail as it is.
    if (element === null || list === null || list === undefined || typeof ResizeObserver === 'undefined') return;
    const fit = () => {
      element.removeAttribute('data-tight');
      if (list.getBoundingClientRect().right > element.getBoundingClientRect().right + 0.5) element.setAttribute('data-tight', '');
    };
    const watch = new ResizeObserver(fit);
    watch.observe(element);
    void document.fonts.ready.then(fit);
    return () => {
      watch.disconnect();
    };
  }, [drawn]);
  return rail;
}

interface StepRailProps {
  steps: readonly RailStep[];
  onGo: (slug: string) => void;
}

/**
 * The journey, numbered. The step on screen is named in ink, with a
 * two-pixel ink line under it that sits on the top bar's own rule. A step
 * that is done shows a tick; one to come is grey. A step that can be
 * pressed is a button; the others are not.
 *
 * A step whose name is folded away (a done step on a crowded rail; any
 * step but the current one in a narrow window, or whenever the names do
 * not fit) keeps its name as a tooltip. What is spoken is always whole:
 * the number, the name, and whether the step is done.
 */
export function StepRail({ steps, onGo }: StepRailProps) {
  const rail = useTightRail(steps);
  return (
    <nav ref={rail} className="step-rail" aria-label={SHELL_WORDS.stepsLabel} data-crowded={steps.length > ROOMY || undefined}>
      <ol>
        {steps.map((step, at) => {
          const face = (
            <>
              <span className="step-rail-number" aria-hidden="true">
                {step.state === 'done' ? <Icon name="check" /> : step.number}
              </span>
              <span className="step-rail-title" aria-hidden="true">
                {step.title}
              </span>
              <span className="step-rail-spoken">{SHELL_WORDS.step(step.number, step.title, step.state === 'done')}</span>
            </>
          );
          return (
            <Fragment key={step.slug}>
              {at > 0 && <li className="step-rail-link" data-done={steps[at - 1]?.state === 'done' || undefined} aria-hidden="true" />}
              <li className="step-rail-step" data-state={step.state}>
                <Tooltip tip={step.title} spoken={false}>
                  {step.open ? (
                    <button
                      type="button"
                      className="step-rail-face"
                      onClick={() => {
                        onGo(step.slug);
                      }}
                    >
                      {face}
                    </button>
                  ) : (
                    <span className="step-rail-face" aria-current={step.state === 'now' ? 'step' : undefined}>
                      {face}
                    </span>
                  )}
                </Tooltip>
              </li>
            </Fragment>
          );
        })}
      </ol>
    </nav>
  );
}
