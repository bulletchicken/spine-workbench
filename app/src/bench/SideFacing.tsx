import { caseStore } from '@/case-state/case-store';
import { filmPixels } from '@/case-state/film-pixels';
import { filmOf, marksOf } from '@/case-state/selectors';
import { fitSources } from '@/view-state/fit-sources';
import { findState } from '@/view-state/find-state';
import { SIDE_WORDS as W } from '@/words/side';
import { clearSpineBuild } from './three/build-spine';
import './SideFacing.css';

export function answerSideFacing(opening: number, imageRight: 'posterior' | 'anterior'): void {
  if (filmPixels.getState().openings.side !== opening) return;
  const { case: c, version } = caseStore.getState();
  const film = filmOf(c, 'side');
  if (film === null) return;
  const marks = marksOf(c, 'side');
  clearSpineBuild();
  fitSources.getState().forget('side');
  findState.getState().clear();
  caseStore.getState().apply([
    { kind: 'set-geometry', film: 'side', geometry: { ...film.geometry, imageRight, confirmed: { ...film.geometry.confirmed, facing: true } } },
    ...(marks === null ? [] : [{ kind: 'replace-marks' as const, marks: { ...marks, vertebrae: [], landmarks: { c7: null, sacrumTop: null, hips: [] } }, basedOn: version + 1, cause: { by: 'auto-pass' as const } }]),
  ]);
}

export function SideFacing({ opening }: { opening: number }) {
  return <fieldset className="side-controls" onClick={(event) => { event.stopPropagation(); }}>
    <legend>{W.facing}</legend>
    <label><input type="radio" name={`side-facing-${String(opening)}`} onChange={() => { answerSideFacing(opening, 'posterior'); }} />{W.left}</label>
    <label><input type="radio" name={`side-facing-${String(opening)}`} onChange={() => { answerSideFacing(opening, 'anterior'); }} />{W.right}</label>
  </fieldset>;
}
