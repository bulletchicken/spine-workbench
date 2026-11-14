import { caseStore } from '@/case-state/case-store';
import { filmPixels } from '@/case-state/film-pixels';
import type { EngineClient } from '@/engine-room/engine-client';
import { TEMPLATE_LEVELS, type CaseFileV1, type SpineFit } from '@/shapes';
import { loadBones } from '@/spine-3d/bones/load-bones';
import { currentStep } from '@/view-state/current-step';
import { fitSources } from '@/view-state/fit-sources';
import { spineBuild } from '@/view-state/spine-build';
import { threeState } from '@/view-state/three-state';
import { fitInputFor, fitKeyFor } from './fit-input';

let run = 0;
function keyFor(c: CaseFileV1): string | null {
  try { return fitKeyFor(c, fitSources.getState().films, filmPixels.getState().openings); }
  catch { return null; }
}
export function hasCurrentFit(c: CaseFileV1): boolean {
  const accepted = spineBuild.getState().accepted;
  return accepted !== null && accepted.key === keyFor(c);
}
function finite(value: unknown): boolean {
  if (typeof value === 'number') return Number.isFinite(value);
  if (Array.isArray(value)) return value.every(finite);
  if (value !== null && typeof value === 'object') return Object.values(value).every(finite);
  return true;
}
function usableFit(fit: SpineFit): boolean {
  return fit.converged && finite(fit) && fit.vertebrae.some((v) => v.level !== 'C7' && TEMPLATE_LEVELS.standard.includes(v.level)) && fit.vertebrae.every((v) => v.pose.height > 0 && v.pose.topHalfWidth > 0 && v.pose.bottomHalfWidth > 0 && v.pose.topHalfDepth > 0 && v.pose.bottomHalfDepth > 0);
}
export function cancelSpineBuild(): void { run++; spineBuild.getState().cancel(); }
export function clearSpineBuild(): void { cancelSpineBuild(); spineBuild.getState().clear(); threeState.getState().clear(); }
export function failSpineView(): void {
  cancelSpineBuild();
  spineBuild.getState().fail();
  currentStep.getState().go('find');
}
export async function buildSpine(engine: Pick<EngineClient, 'fitSpine'>): Promise<void> {
  if (spineBuild.getState().phase === 'building') return;
  const c = caseStore.getState().case;
  if (hasCurrentFit(c)) { threeState.getState().clear(); currentStep.getState().go('three'); return; }
  const mine = ++run;
  spineBuild.getState().begin();
  const key = keyFor(c);
  const stillCurrent = () => mine === run && currentStep.getState().slug === 'find' && key !== null && keyFor(caseStore.getState().case) === key;
  const timer = setTimeout(() => { if (stillCurrent()) spineBuild.getState().markBusy(); }, 300);
  try {
    const input = fitInputFor(c, fitSources.getState().films, filmPixels.getState().openings);
    const [fit, bones] = await Promise.all([engine.fitSpine(input), loadBones()]);
    if (!stillCurrent()) return;
    if (!usableFit(fit) || !fit.vertebrae.some((v) => v.level !== 'C7' && TEMPLATE_LEVELS.standard.includes(v.level) && bones.some((b) => b.level === v.level))) throw new Error('fit-has-no-drawable-spine');
    spineBuild.getState().accept({ key: key ?? '', input, fit, bones });
    threeState.getState().clear();
    currentStep.getState().go('three');
  } catch {
    if (mine === run && currentStep.getState().slug === 'find' && (key === null || stillCurrent())) spineBuild.getState().fail();
  } finally {
    clearTimeout(timer);
    if (mine === run) spineBuild.getState().cancel();
  }
}
