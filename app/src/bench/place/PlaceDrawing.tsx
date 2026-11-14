import { useEffect, useLayoutEffect, useMemo, useRef, type RefObject } from 'react';
import { useStore } from 'zustand';
import { useKey } from '@/building-blocks/keyboard/use-key';
import { takeKeys } from '@/building-blocks/keyboard/keyboard';
import { joinLoop } from '@/choreography/one-loop';
import { SYSTEM_ASKS_FOR_REDUCED } from '@/choreography/reduced-motion';
import { durationOf } from '@/choreography/timing';
import type { Spine3DHandle } from '@/spine-3d/view/Spine3D';
import type { AcceptedSpine } from '@/view-state/spine-build';
import { threeState } from '@/view-state/three-state';
import { PLACE_WORDS as W } from '@/words/place';
import { drawPlacement, PLACE_DRAWING as D } from './draw-placement';
import { filmSide, placementLooks, reviewItems } from './placement-view';
import { placeState } from './place-state';
import { restingView } from '../three/resting-view';
import { PlacementCaption } from './PlacementCaption';
import './place.css';

export function PlaceDrawing({ accepted, handle }: { accepted: AcceptedSpine; handle: RefObject<Spine3DHandle | null> }) {
  const state = useStore(placeState), root = useRef<HTMLDivElement>(null), chip = useRef<HTMLDivElement>(null);
  const veiling = useRef(false);
  const result = state.result, layout = result?.layouts[0];
  const items = useMemo(() => result === null ? [] : reviewItems(result, accepted.input.front.marks.namedFor), [result, accepted.input.front.marks.namedFor]);
  const item = state.review === null ? undefined : items[state.review];
  const looks = useMemo(() => layout === undefined ? null : placementLooks(layout), [layout]);
  const selected = state.pressed ?? state.hovered;
  const anchor = selected?.startsWith('anchor:') === true ? layout?.anchors[Number(selected.slice(7))] : undefined;
  const rod = selected?.startsWith('rod:') === true ? layout?.rods.find((r) => `rod:${r.side}` === selected) : undefined;
  const chosenEnd = selected === 'upper' ? layout?.upper : selected === 'lower' ? layout?.lower : undefined;
  const levels = useMemo(() => item?.levels ?? (anchor === undefined ? chosenEnd === undefined ? [] : [chosenEnd] : [anchor.level]), [item, anchor, chosenEnd]);
  useLayoutEffect(() => { threeState.getState().light(levels); return () => { threeState.getState().light([]); }; }, [levels]);
  useLayoutEffect(() => {
    const on = handle.current, spine = on?.spine(), stage = on?.stage();
    if (layout === undefined || on == null || spine == null || stage == null) return;
    const view = stage.view(); on.lookFrom({ ...view, azimuth: D.backAzimuth, elevation: D.backElevation, perspective: 1 });
    const surface = root.current;
    const fade = surface === null ? 0 : durationOf(surface, '--dur-appear');
    const first = spine.levels[0], from = first === undefined ? 1 : spine.valuesOf(first)?.strength ?? 1;
    let elapsed = 0; veiling.current = true;
    const stop = joinLoop((dt) => {
      elapsed += dt;
      const progress = fade === 0 ? 1 : Math.min(1, elapsed / fade);
      const review = placeState.getState().review;
      const target = review === null || items[review]?.levels.length === 0 ? D.bone : D.reviewBone;
      spine.set({ strength: from + (target - from) * progress });
      if (progress === 1) { veiling.current = false; stop(); }
    });
    return () => { stop(); veiling.current = false; spine.set({ strength: 1 }); on.lookFrom(view, { animate: false }); placeState.setState({ hovered: null, pressed: null }); };
  }, [handle, layout, items]);
  useLayoutEffect(() => {
    const on = handle.current, stage = on?.stage(), surface = root.current;
    if (on == null || stage == null || surface === null) return;
    let size = stage.size(), fitted = restingView(stage, surface);
    return stage.onDrawn(() => {
      const next = stage.size();
      if (next.width === size.width && next.height === size.height) return;
      const from = stage.view(), to = restingView(stage, surface);
      const across = from.pxPerMm[0] / fitted.pxPerMm[0], up = from.pxPerMm[1] / fitted.pxPerMm[1];
      size = next; fitted = to;
      on.lookFrom({ ...from, aimAt: to.aimAt, pxPerMm: [to.pxPerMm[0] * across, to.pxPerMm[1] * up] }, { animate: false });
    });
  }, [handle]);
  useLayoutEffect(() => {
    const on = handle.current, spine = on?.spine(), stage = on?.stage(), surface = root.current;
    if (layout === undefined || spine == null || stage == null || surface === null) return;
    const marks = result?.firstIsSketch === true ? [] : result?.cannotTell.filter((r) => r.code === 'ends-unsettled').flatMap((r) => r.levels) ?? [];
    const drawn = drawPlacement(spine, accepted.fit, layout, marks);
    if (!placeState.getState().arriving) return () => { drawn.remove(); };
    const reduced = window.matchMedia(SYSTEM_ASKS_FOR_REDUCED).matches || surface.closest('[data-motion="reduced"]') !== null;
    const fade = durationOf(surface, '--dur-appear');
    let elapsed = 0, finished = false;
    const finish = () => { if (finished) return; finished = true; drawn.reveal(drawn.parts.length, 1); spine.set({ strength: D.bone }); placeState.setState({ arriving: false }); };
    drawn.reveal(0, 0);
    const stop = joinLoop((dt) => {
      elapsed += dt;
      const total = reduced ? fade : Math.max(D.arrivalMs, drawn.parts.length * D.staggerMs);
      drawn.reveal(reduced ? drawn.parts.length : Math.floor(elapsed / D.staggerMs) + 1, Math.min(1, elapsed / fade));
      if (elapsed >= total) { stop(); finish(); }
    });
    const unkey = takeKeys('gesture', ['Enter'], () => { stop(); finish(); }, () => !finished);
    const skip = (event: PointerEvent) => { if (finished || event.target instanceof Element && event.target.closest('.steps') !== null) return; event.preventDefault(); event.stopPropagation(); stop(); finish(); };
    document.addEventListener('pointerdown', skip, true);
    return () => { stop(); unkey(); document.removeEventListener('pointerdown', skip, true); drawn.remove(); };
  }, [accepted, handle, layout, result]);
  useEffect(() => {
    const on = handle.current, spine = on?.spine(), stage = on?.stage();
    if (layout === undefined || on == null || spine == null || stage == null) return;
    if (!veiling.current) spine.set({ strength: item === undefined || item.levels.length === 0 ? D.bone : D.reviewBone });
    if (item !== undefined) {
      const at = item.anchor === null ? undefined : layout.anchors[item.anchor];
      const azimuth = at === undefined ? D.backAzimuth : Math.atan2(-at.directionUnit[1], -at.directionUnit[0]) * 180 / Math.PI;
      on.lookFrom({ ...stage.view(), azimuth, elevation: D.backElevation, perspective: 1 });
    } else if (state.chosen !== null) {
      const view = state.chosen === 'back' ? { ...stage.view(), azimuth: D.backAzimuth, elevation: D.backElevation, perspective: 1 } : { ...stage.namedView(state.chosen), pxPerMm: stage.view().pxPerMm };
      on.lookFrom(view);
    }
  }, [handle, item, layout, state.chosen]);
  useKey('popover', ['Escape'], () => { placeState.setState({ pressed: null, hovered: null }); }, () => state.pressed !== null && !state.sheet);
  useEffect(() => {
    if (state.pressed === null) return;
    const outside = (event: PointerEvent) => { if (!(event.target instanceof Element) || event.target.closest('.three-stage') === null) placeState.setState({ pressed: null, hovered: null }); };
    document.addEventListener('pointerdown', outside, true);
    return () => { document.removeEventListener('pointerdown', outside, true); };
  }, [state.pressed]);
  const partIds = useMemo(() => layout === undefined ? [] : ['upper', ...layout.anchors.map((_, i) => `anchor:${String(i)}`), 'lower', ...layout.rods.filter((r) => r.standingPointsMm.length > 1).map((r) => `rod:${r.side}`)], [layout]);
  useKey('gesture', ['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight', 'Enter', 'Escape'], (event) => {
    if (event.key === 'Escape') { placeState.setState({ pressed: null, hovered: null, review: null }); return; }
    if (event.key === 'Enter') { placeState.setState({ pressed: selected ?? partIds[0] ?? null }); return; }
    let index = partIds.indexOf(selected ?? '');
    if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
      if (anchor !== undefined && layout !== undefined) {
        const opposite = layout.anchors.findIndex((a) => a.level === anchor.level && a.side !== anchor.side);
        if (opposite >= 0) index = partIds.indexOf(`anchor:${String(opposite)}`);
      } else if (rod !== undefined) {
        const other = partIds.indexOf(`rod:${rod.side === 'left' ? 'right' : 'left'}`);
        if (other >= 0) index = other;
      }
    } else if (anchor !== undefined && layout !== undefined) {
      const sameSide = layout.anchors.map((a, i) => ({ a, i })).filter(({ a }) => a.side === anchor.side);
      const at = sameSide.findIndex(({ a }) => a === anchor);
      const next = sameSide[at + (event.key === 'ArrowUp' ? -1 : 1)];
      index = partIds.indexOf(next === undefined ? event.key === 'ArrowUp' ? 'upper' : 'lower' : `anchor:${String(next.i)}`);
    } else index = Math.max(0, Math.min(partIds.length - 1, index + (event.key === 'ArrowUp' ? -1 : 1)));
    const next = partIds[index];
    if (next !== undefined && next !== selected) placeState.setState({ hovered: next, pressed: null });
  }, () => !state.arriving && !state.sheet && document.activeElement?.classList.contains('three-stage') === true);
  useLayoutEffect(() => {
    const stage = handle.current?.stage(), spine = handle.current?.spine(), surface = root.current;
    if (stage == null || spine == null || surface === null || layout === undefined) return;
    const project = () => {
      const first = spine.levels[0];
      if (first !== undefined) surface.dataset.boneStrength = String(spine.valuesOf(first)?.strength);
      for (const node of surface.querySelectorAll<SVGElement>('[data-anchor]')) {
        const a = layout.anchors[Number(node.dataset.anchor)]; if (a === undefined) continue;
        const p = stage.patientToCanvas(a.entryMm, spine); node.setAttribute('transform', `translate(${String(p.x)} ${String(p.y)})`);
      }
      for (const node of surface.querySelectorAll<SVGElement>('[data-end]')) {
        const upper = node.dataset.end === 'upper', level = upper ? layout.upper : layout.lower;
        const marks = spine.landmarks(level); if (marks === null) continue;
        const plate = upper ? marks.top : marks.bottom;
        const point = stage.toCanvas(plate.centre);
        const left = stage.toCanvas(plate.left), right = stage.toCanvas(plate.right);
        const gap = parseFloat(getComputedStyle(surface).getPropertyValue('--space-8'));
        node.querySelector('text')?.setAttribute('x', String(Math.max(left.x, right.x) - point.x + gap));
        node.setAttribute('transform', `translate(${String(point.x)} ${String(point.y)})`);
      }
      let point = anchor?.entryMm ?? rod?.standingPointsMm[0];
      if (point === undefined && chosenEnd !== undefined) point = accepted.fit.vertebrae.find((v) => v.level === chosenEnd)?.pose.centre;
      if (point !== undefined && chip.current !== null) {
        const p = stage.patientToCanvas(point, spine), css = getComputedStyle(surface);
        const dx = parseFloat(css.getPropertyValue('--chip-dx')), dy = parseFloat(css.getPropertyValue('--chip-dy'));
        chip.current.style.left = `${String(Math.max(0, Math.min(stage.size().width - chip.current.offsetWidth, p.x + dx)))}px`;
        chip.current.style.top = `${String(Math.max(0, Math.min(stage.size().height - chip.current.offsetHeight, p.y + dy)))}px`;
      }
    };
    project(); return stage.onDrawn(project);
  }, [accepted, anchor, chosenEnd, handle, layout, rod, state.sheet, state.arriving]);
  const facing = accepted.input.front.marks.namedFor;
  const title = anchor === undefined ? rod === undefined ? chosenEnd === undefined ? null : W.end(selected === 'upper', chosenEnd) : W.rod(filmSide(rod.side, facing)) : W.part(anchor.level, filmSide(anchor.side, facing), anchor.kind);
  const width = anchor?.pedicle.outerWidthMm;
  const wider = anchor?.kind === 'screw' ? anchor.widerThanPedicle.find((count) => count.diameterMm === anchor.diameterMm) : undefined;
  return <div className="place-drawing" ref={root} aria-hidden={state.sheet || undefined}>
    {layout !== undefined && !state.arriving && <PlacementCaption />}
    {layout !== undefined && <svg className="place-labels" style={{ opacity: state.arriving ? 0 : 1 }} aria-hidden="true">
      {layout.anchors.map((_, i) => (looks?.local[i]?.length ?? 0) > 0 || selected === `anchor:${String(i)}` || item?.anchor === i ? <g key={i} data-anchor={i}><circle className={selected === `anchor:${String(i)}` ? 'pointed' : 'look'} r={D.ringRadiusPx} /></g> : null)}
      {([['upper', layout.upper], ['lower', layout.lower]] as const).map(([end, level]) => <g key={end} data-end={end}><text x="24" className="num">{level}</text>{result?.firstIsSketch !== true && result?.cannotTell.some((r) => r.code === 'ends-unsettled' && r.levels.includes(level)) === true && <circle className="look" r={D.ringRadiusPx} />}</g>)}
    </svg>}
    {title !== null && !state.arriving && !state.sheet && <div ref={chip} className="place-chip glass" role="status">
      <div>{title}</div>
      {anchor?.diameterMm != null && anchor.lengthMm !== null && <div className="num">{W.size(anchor.diameterMm, anchor.lengthMm)}</div>}
      {rod !== undefined && <div className="num">{W.diameter(rod.diameterMm)}</div>}
      {anchor?.kind === 'screw' && (anchor.diameterAlsoSeen.length > 0 || anchor.lengthAlsoSeen.length > 0) && <p>{W.also(anchor.diameterAlsoSeen, anchor.lengthAlsoSeen)}</p>}
      {wider !== undefined && wider.versions > 0 && result !== null && <p>{W.wider(wider.versions, result.versions)}</p>}
      {anchor !== undefined && width != null && <p>{W.pedicle(width.p10, width.p90)}{W.origin}</p>}
    </div>}
  </div>;
}
