import { SourceLinks } from './PlaceSources';
import { pedicleOrigin,sourceUses,endSourceKeys } from './placement-sources';
import { useEffect, useLayoutEffect, useMemo, useRef, type RefObject } from 'react';
import { useStore } from 'zustand';
import { LevelWords } from '@/building-blocks/LevelWords';
import { useKey } from '@/building-blocks/keyboard/use-key';
import { takeKeys } from '@/building-blocks/keyboard/keyboard';
import { joinLoop } from '@/choreography/one-loop';
import { SYSTEM_ASKS_FOR_REDUCED } from '@/choreography/reduced-motion';
import { durationOf } from '@/choreography/timing';
import type { Spine3DHandle } from '@/spine-3d/view/Spine3D';
import type { AcceptedSpine } from '@/view-state/spine-build';
import { threeState } from '@/view-state/three-state';
import { DAILY_STOPS_NOT_SHOWN, ESTIMATE_NAMES, ESTIMATE_TAGS, PLACE_LOAD_WORDS as L, PLACE_WORDS as W, PLACE_EDIT_WORDS as importedEditWords } from '@/words/place';
import { drawPlacement, type DrawnPlacement, PLACE_DRAWING as D } from './draw-placement';
import { filmSide, placementLooks, reviewItems } from './placement-view';
import { placeState } from './place-state';
import { restingView } from '../three/resting-view';
import { loadState } from './load-state';
import { anchorLoad, anchorPoint, loadLabels, pastWords, rangeWords, ringedPast, seatPoint, shownLoadStop, shownStopResult,tooFewSettled } from './load-view';
import { useLoadDrawing } from './load-drawing';
import { RangeBar, scaleFor } from './LoadEstimates';
import { PartControls } from './PartControls';
import { EndHandles, endTextX } from './EndHandles';
import { changePlacement, fittedLevels, handState, openWhy, snapEnd } from './hand-state';
import { useEngine } from '@/engine-room/use-engine';
import { PlacementCaption } from './PlacementCaption';
import './place.css';

export function PlaceDrawing({ accepted, handle }: { accepted: AcceptedSpine; handle: RefObject<Spine3DHandle | null> }) {
  const state = useStore(placeState), root = useRef<HTMLDivElement>(null), chip = useRef<HTMLDivElement>(null);
  const veiling = useRef(false), drawnRef=useRef<DrawnPlacement|null>(null);
  const load=useStore(loadState);
  const stop=shownLoadStop(load.stop),loaded=load.phase==='loaded',currentLoad=loaded?shownStopResult(load.result,stop):undefined;
  const loadParts=stop==='before'||tooFewSettled(load.result)?[]:currentLoad?.parts??[];
  const result = state.result, layout = result?.layouts[0];
  const items = useMemo(() => result === null ? [] : reviewItems(result, accepted.input.front.marks.namedFor), [result, accepted.input.front.marks.namedFor]);
  const itemsRef=useRef(items);useLayoutEffect(()=>{itemsRef.current=items;},[items]);
  const item = state.review === null ? undefined : items[state.review];
  const looks = useMemo(() => layout === undefined ? null : placementLooks(layout), [layout]);
  const selected = state.pressed ?? state.hovered;
  const hasLayout=layout!==undefined;
  const anchor = selected?.startsWith('anchor:') === true ? layout?.anchors[Number(selected.slice(7))] : undefined;
  const span=selected?.startsWith('span:')===true?selected.split(':'):null;
  const rod = span!==null?layout?.rods.find((r)=>r.side===span[1]):selected?.startsWith('rod:') === true ? layout?.rods.find((r) => `rod:${r.side}` === selected) : undefined;
  const selectedLoad=anchor===undefined?loadParts.find((p)=>p.part.kind==='rod-span'?selected===`span:${p.part.side}:${p.part.from}:${p.part.to}`:p.part.kind==='rod'&&selected===`rod:${p.part.side}`):anchorLoad(loadParts,anchor);
  const chosenEnd = selected === 'upper' ? layout?.upper : selected === 'lower' ? layout?.lower : undefined;
  const levels = useMemo(() => {
    if(item!==undefined)return item.levels;
    if(anchor!==undefined)return [anchor.level];
    if(chosenEnd!==undefined)return [chosenEnd];
    const ids=selected?.startsWith('span:')===true?selected.split(':'):[];
    return ids.slice(2).filter((level)=>accepted.fit.vertebrae.some((v)=>v.level===level)) as import('@/shapes').Level[];
  }, [item, anchor, chosenEnd, selected, accepted.fit]);
  useLayoutEffect(() => { threeState.getState().light(levels); return () => { threeState.getState().light([]); }; }, [levels]);
  useLayoutEffect(() => {
    const on = handle.current, spine = on?.spine(), stage = on?.stage();
    if (!hasLayout || on == null || spine == null || stage == null) return;
    const view = stage.view(); on.lookFrom({ ...view, azimuth: D.backAzimuth, elevation: D.backElevation, perspective: 1 });
    const surface = root.current;
    const fade = surface === null ? 0 : durationOf(surface, '--dur-appear');
    const first = spine.levels[0], from = first === undefined ? 1 : spine.valuesOf(first)?.strength ?? 1;
    let elapsed = 0; veiling.current = true;
    const stop = joinLoop((dt) => {
      elapsed += dt;
      const progress = fade === 0 ? 1 : Math.min(1, elapsed / fade);
      const review = placeState.getState().review;
      const target = review === null || itemsRef.current[review]?.levels.length === 0 ? D.bone : D.reviewBone;
      spine.set({ strength: from + (target - from) * progress });
      if (progress === 1) { veiling.current = false; stop(); }
    });
    return () => { stop(); veiling.current = false; spine.set({ strength: 1 }); on.lookFrom(view, { animate: false }); placeState.setState({ hovered: null, pressed: null }); };
  }, [handle, hasLayout]);
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
    const drawn = drawPlacement(spine, accepted.fit, layout, marks); drawnRef.current=drawn;
    if (!placeState.getState().arriving) {
      const changes=handState.getState().flashing;
      if(changes.length===0)return () => { drawn.remove(); drawnRef.current=null; };
      let elapsed=0;drawn.fadeParts(changes,0);const fade=durationOf(surface,'--dur-appear');
      const stop=joinLoop((dt)=>{elapsed+=dt;const p=fade===0?1:Math.min(1,elapsed/fade);drawn.fadeParts(changes,p);if(p===1){stop();handState.setState({flashing:[]});}});
      return ()=>{stop();drawn.remove();drawnRef.current=null;};
    }
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
    return () => { stop(); unkey(); document.removeEventListener('pointerdown', skip, true); drawn.remove(); drawnRef.current=null; };
  }, [accepted, handle, layout, result]);
  useLayoutEffect(()=>{
    const on=handle.current,stage=on?.stage(),surface=root.current,dock=surface?.closest('.frame, .place-gallery')?.querySelector('.dock');
    if(layout===undefined||on==null||stage==null||surface==null||dock==null)return;
    let previous='',wasNarrow=false;
    const fit=()=>{
      // A wide window keeps the resting view: nothing on the page needs reading, frame after frame.
      const narrow=window.innerWidth<1100;if(!narrow&&!wasNarrow)return;
      const size=stage.size(),box=surface.getBoundingClientRect(),top=dock.getBoundingClientRect().top;
      const key=`${String(size.width)}:${String(size.height)}:${String(top)}`;if(key===previous)return;previous=key;
      wasNarrow=narrow;
      const caption=surface.querySelector('.place-caption'),gap=parseFloat(getComputedStyle(surface).getPropertyValue('--space-12'));
      const clear=narrow?Math.max(1,Math.min(size.height,top-box.top-(caption?.getBoundingClientRect().height??0)-2*gap)):size.height;
      const base=restingView(stage,surface),view=stage.view(),scale=Math.min(1,clear/size.height);
      on.lookFrom({...view,aimAt:[size.width/2,clear/2],pxPerMm:[base.pxPerMm[0]*scale,base.pxPerMm[1]*scale]},{animate:false});
    };
    fit();const observer=new ResizeObserver(fit);observer.observe(dock);observer.observe(surface);const unstage=stage.onDrawn(fit);
    return ()=>{observer.disconnect();unstage();};
  },[handle,layout]);
  const motion=useLoadDrawing(accepted,handle,drawnRef,root);
  const labels=useMemo(()=>result===null||load.result===null?[]:loadLabels(load.result,result,accepted.fit,shownLoadStop(load.stop),DAILY_STOPS_NOT_SHOWN?'rods-in':'bending-forward'),[result,load.result,load.stop,accepted.fit]);
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
  useKey('popover', ['Escape'], () => { placeState.setState({ pressed: null, hovered: null, keyboard:true });root.current?.closest<HTMLElement>('.three-stage')?.focus(); }, () => state.pressed !== null && !state.sheet);
  useEffect(() => {
    if (state.pressed === null) return;
    const outside = (event: PointerEvent) => { if (!(event.target instanceof Element) || event.target.closest('.three-stage') === null) placeState.setState({ pressed: null, hovered: null }); };
    document.addEventListener('pointerdown', outside, true);
    return () => { document.removeEventListener('pointerdown', outside, true); };
  }, [state.pressed]);
  const partIds = useMemo(() => layout === undefined ? [] : ['upper', ...layout.anchors.map((_, i) => `anchor:${String(i)}`), 'lower', ...layout.rods.filter((r) => r.standingPointsMm.length > 1).map((r) => `rod:${r.side}`)], [layout]);
  useKey('gesture', ['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight', 'Enter', 'Escape'], (event) => {
    placeState.setState({keyboard:true});
    if (event.key === 'Escape') { placeState.setState({ pressed: null, hovered: null, review: null }); return; }
    if (event.key === 'Enter') { const id=selected??partIds[0]??null;if(id==='upper'||id==='lower')openWhy(id);else placeState.setState({pressed:id});return; }
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
    // Read once, and again when they change size: the page's spacing, the chip's offset from what it names, where the labels' sheet
    // sits in the drawing, how far down the dock leaves the drawing clear, and the chip's size. Nothing on the page is read again frame
    // after frame. A change of size places the chip at once: a chip that grows into its controls draws no frame of the spine.
    let gap = 0, chipDx = 0, chipDy = 0, clear = Infinity, sheet: { x: number; y: number } | null = null, last = '', was: readonly unknown[] = [];
    const placed = new WeakSet<Element>();
    let chipSeen: HTMLDivElement | null = null, chipSize: { offsetWidth: number; offsetHeight: number; width: number; height: number } | null = null;
    const dock = surface.closest('.frame, .place-gallery')?.querySelector('.dock') ?? null;
    const readStyle = () => {
      const css = getComputedStyle(surface); gap = parseFloat(css.getPropertyValue('--space-8')); chipDx = parseFloat(css.getPropertyValue('--chip-dx')); chipDy = parseFloat(css.getPropertyValue('--chip-dy'));
      // The dock lies over the foot of the drawing: the chip ends a gap above it.
      const below = dock?.getBoundingClientRect(); clear = below === undefined || below.height === 0 ? Infinity : below.top - surface.getBoundingClientRect().top - gap;
      sheet = null; last = '';
    };
    readStyle();
    const resized = new ResizeObserver(() => { readStyle(); project(); }), chipResized = new ResizeObserver(() => { chipSize = null; last = ''; project(); });
    resized.observe(surface); const sheetNode = surface.querySelector('.place-labels'); if (sheetNode !== null) resized.observe(sheetNode); if (dock !== null) resized.observe(dock);
    const project = () => {
      const first = spine.levels[0];
      const strength = first === undefined ? undefined : String(spine.valuesOf(first)?.strength);
      if (strength !== undefined && surface.dataset.boneStrength !== strength) surface.dataset.boneStrength = strength;
      const chipNow = chip.current;
      if (chipNow !== chipSeen) { if (chipSeen !== null) chipResized.unobserve(chipSeen); chipSeen = chipNow; chipSize = null; if (chipNow !== null) chipResized.observe(chipNow); }
      // Every read of the page comes first, then every write.
      if (chipNow !== null && chipSize === null) { const rect = chipNow.getBoundingClientRect(); chipSize = { offsetWidth: chipNow.offsetWidth, offsetHeight: chipNow.offsetHeight, width: rect.width, height: rect.height }; }
      // Nothing is moved when neither the view, the spine, the poses, the hardware nor the chip has, and every mark is one already placed.
      const view = stage.view(), size = stage.size(), room = Math.max(0, Math.min(size.height, clear)), chipWidth = chipSize?.offsetWidth ?? 0, chipHeight = Math.min(room, chipSize?.offsetHeight ?? 0);
      const key = [view.azimuth, view.elevation, view.perspective, ...view.pxPerMm, ...view.aim, ...view.aimAt, size.width, size.height, room, chipWidth, chipHeight].join(), now = [spine.bounds(), motion.poses.current, drawnRef.current, chipNow];
      const marks = surface.querySelectorAll<SVGElement>('[data-anchor], [data-end], [data-load-part], [data-range]');
      if (key === last && !motion.moving.current && now.every((part, i) => part === was[i]) && [...marks].every((mark) => placed.has(mark))) return;
      last = key; was = now; for (const mark of marks) placed.add(mark);
      const ranges = [...surface.querySelectorAll<SVGGElement>('[data-range]')];
      let point = anchor===undefined?rod===undefined?undefined:layout.anchors.filter((a)=>a.side===rod.side&&a.kind!=='none').map((a)=>anchorPoint(a,accepted.fit,motion.poses.current))[0]:anchorPoint(anchor,accepted.fit,motion.poses.current);
      const spanId=selected?.startsWith('span:')===true?selected.split(':'):[];
      const from=layout.anchors.find((a)=>a.side===spanId[1]&&a.level===spanId[2]),to=layout.anchors.find((a)=>a.side===spanId[1]&&a.level===spanId[3]);
      if(from!==undefined&&to!==undefined){const a=anchorPoint(from,accepted.fit,motion.poses.current),b=anchorPoint(to,accepted.fit,motion.poses.current);point=[(a[0]+b[0])/2,(a[1]+b[1])/2,(a[2]+b[2])/2];}
      if(rod!==undefined)point=drawnRef.current?.point(selected??'')??point;
      if (point === undefined && chosenEnd !== undefined) point = accepted.fit.vertebrae.find((v) => v.level === chosenEnd)?.pose.centre;
      /** Boxes from the drawing's top-left corner, for the labels the chip would cover. */
      const covered = chipNow !== null && chipSize !== null && ranges.length > 0;
      const unplaced = new Map<SVGGElement, { left: number; right: number; top: number; bottom: number } | undefined>();
      let chipAt: { left: number; right: number; top: number; bottom: number } | undefined;
      // Rarely a label or the chip is not moved this frame: it stays where the page has it, read before anything is written.
      const still = covered ? ranges.filter((node) => { const label = labels.find((l) => l.id === node.dataset.range); return label === undefined || spine.centre(label.level) === null; }) : [];
      if (covered && (sheet === null || still.length > 0 || point === undefined)) {
        const surfaceBox = surface.getBoundingClientRect();
        const fromSurface = (box: DOMRect) => ({ left: box.left - surfaceBox.left, right: box.right - surfaceBox.left, top: box.top - surfaceBox.top, bottom: box.bottom - surfaceBox.top });
        if (sheet === null) { const on = sheetNode?.getBoundingClientRect(); sheet = { x: (on?.left ?? surfaceBox.left) - surfaceBox.left, y: (on?.top ?? surfaceBox.top) - surfaceBox.top }; }
        for (const node of still) { const box = node.querySelector('foreignObject')?.getBoundingClientRect(); unplaced.set(node, box === undefined ? undefined : fromSurface(box)); }
        if (point === undefined) chipAt = fromSurface(chipNow.getBoundingClientRect());
      }
      for (const node of surface.querySelectorAll<SVGElement>('[data-anchor]')) {
        const a = layout.anchors[Number(node.dataset.anchor)]; if (a === undefined) continue;
        const p = stage.patientToCanvas(anchorPoint(a,accepted.fit,motion.poses.current), spine); node.setAttribute('transform', `translate(${String(p.x)} ${String(p.y)})`);
      }
      for (const node of surface.querySelectorAll<SVGElement>('[data-end]')) {
        const upper = node.dataset.end === 'upper', level = upper ? layout.upper : layout.lower;
        const marks = spine.landmarks(level); if (marks === null) continue;
        const plate = upper ? marks.top : marks.bottom;
        const point = stage.toCanvas(plate.centre);
        const left = stage.toCanvas(plate.left), right = stage.toCanvas(plate.right);
        node.querySelector('text')?.setAttribute('x', String(endTextX(stage, spine, [left.x, right.x], point.y, drawnRef.current?.reach(upper ? 'upper' : 'lower') ?? [], (layout.rods[0]?.diameterMm ?? 0) * D.headOverRod / 2, gap) - point.x));
        node.setAttribute('transform', `translate(${String(point.x)} ${String(point.y)})`);
      }
      for(const node of surface.querySelectorAll<SVGGElement>('[data-load-part]')){
        const partId=node.dataset.loadPart??'',whole=drawnRef.current?.point(partId);
        if(whole!=null){const p=stage.patientToCanvas(whole,spine);node.setAttribute('transform',`translate(${String(p.x)} ${String(p.y)})`);continue;}
        const [kind,side,from,to]=partId.split(':');if(kind!=='span')continue;
        const a=layout.anchors.find((a)=>a.side===side&&a.level===from),b=layout.anchors.find((a)=>a.side===side&&a.level===to);
        if(a===undefined)continue;
        const offset=layout.rods.find((r)=>r.side===side)?.headOffsetMm??0,x=stage.patientToCanvas(seatPoint(a,offset,accepted.fit,motion.poses.current),spine),y=b===undefined?x:stage.patientToCanvas(seatPoint(b,offset,accepted.fit,motion.poses.current),spine);
        node.setAttribute('transform',`translate(${String((x.x+y.x)/2)} ${String((x.y+y.y)/2)})`);
      }
      const positions=ranges.flatMap((node)=>{
        const label=labels.find((l)=>l.id===node.dataset.range);if(label===undefined)return[];
        const a=label.anchor===undefined?undefined:layout.anchors[label.anchor];
        const scene=spine.centre(label.level);if(scene===null)return[];
        const at=a===undefined?stage.toCanvas(scene):stage.patientToCanvas(anchorPoint(a,accepted.fit,motion.poses.current),spine);
        return [{node,at,left:label.id==='main-cobb',y:at.y-32}];
      });
      const bounds=spine.bounds();
      const edges=[bounds.low[0],bounds.high[0]].flatMap((x)=>[bounds.low[1],bounds.high[1]].flatMap((y)=>[bounds.low[2],bounds.high[2]].map((z)=>stage.toCanvas([x,y,z]).x)));
      const boxes=new Map<SVGGElement,{left:number;right:number;top:number;bottom:number}>();
      for(const left of [true,false]){
        let previous=16;
        for(const at of positions.filter((p)=>p.left===left).sort((a,b)=>a.y-b.y)){
          const x=left?Math.max(16,Math.min(...edges)-232):Math.min(size.width-224,Math.max(...edges)+24);
          const y=Math.max(previous,Math.min(size.height-112,at.y));previous=y+104;
          at.node.setAttribute('transform',`translate(${String(at.at.x)} ${String(at.at.y)})`);
          const box=at.node.querySelector('foreignObject');box?.setAttribute('x',String(x-at.at.x));box?.setAttribute('y',String(y-at.at.y));
          at.node.querySelector('path')?.setAttribute('d',`M0 0L${String(x-at.at.x+(left?208:0))} ${String(y-at.at.y+24)}`);
          // The label's box is its 208 by 104 sheet, where it was just put.
          if(box!==null&&sheet!==null){const l=sheet.x+at.at.x+(x-at.at.x),t=sheet.y+at.at.y+(y-at.at.y);boxes.set(at.node,{left:l,right:l+208,top:t,bottom:t+104});}
        }
      }
      if (point !== undefined && chipNow !== null) {
        const p = stage.patientToCanvas(point, spine);
        // The whole chip stays in the drawing and above the dock; one taller than that room scrolls within it.
        const left = Math.max(0, Math.min(size.width - chipWidth, p.x + chipDx)), top = Math.max(0, Math.min(room - chipHeight, p.y + chipDy));
        chipNow.style.left = `${String(left)}px`;
        chipNow.style.top = `${String(top)}px`;
        chipNow.style.maxHeight = `${String(room)}px`;
        // The chip is placed from the drawing's own corner: the drawing has no border.
        if (chipSize !== null) chipAt = { left, right: left + chipSize.width, top, bottom: top + Math.min(room, chipSize.height) };
      }
      for (const label of ranges) {
        const box = boxes.get(label) ?? unplaced.get(label);
        const overlaps = covered && chipAt !== undefined && box !== undefined && chipAt.left < box.right && chipAt.right > box.left && chipAt.top < box.bottom && chipAt.bottom > box.top;
        label.style.visibility = overlaps ? 'hidden' : '';
      }
    };
    project(); const unstage = stage.onDrawn(project);
    return () => { unstage(); resized.disconnect(); chipResized.disconnect(); };
  }, [accepted, anchor, chosenEnd, handle, layout, rod, selected, state.sheet, state.arriving, motion.poses, motion.moving, labels]);
  const facing = accepted.input.front.marks.namedFor;
  const title = span!==null&&span[2]!==undefined&&span[3]!==undefined?L.span(span[2] as import('@/shapes').Level,span[3] as import('@/shapes').Level,filmSide(rod?.side??'left',facing)):anchor === undefined ? rod === undefined ? chosenEnd === undefined ? null : W.end(selected === 'upper', chosenEnd) : W.rod(filmSide(rod.side, facing)) : W.part(anchor.level, filmSide(anchor.side, facing), anchor.kind);
  const width = anchor?.pedicle.outerWidthMm;
  const origin=anchor===undefined?null:pedicleOrigin(anchor,state.ageYears);
  const entered=anchor?.diameterStatus==='entered'||anchor?.lengthStatus==='entered';
  const wider = anchor?.kind === 'screw' ? anchor.widerThanPedicle.find((count) => count.diameterMm === anchor.diameterMm) : undefined;
  return <div className="place-drawing" data-load-stop={loaded?stop:undefined} ref={root} aria-hidden={state.sheet || undefined}>
    {state.input!==null&&layout!==undefined&&<EndKeyboard selected={selected} layout={layout}/>}
    {layout !== undefined && !state.arriving && <PlacementCaption />}
    {layout !== undefined && <svg className="place-labels" style={{ opacity: state.arriving ? 0 : 1 }} aria-hidden="true">
      {layout.anchors.map((_, i) => (looks?.local[i]?.length ?? 0) > 0 || (layout.anchors[i]!==undefined&&ringedPast(anchorLoad(loadParts,layout.anchors[i]),load.result)) || selected === `anchor:${String(i)}` || item?.anchor === i ? <g key={i} data-anchor={i}><circle className={selected === `anchor:${String(i)}` ? 'pointed' : 'look'} r={D.ringRadiusPx} /></g> : null)}
      {loadParts.filter((p)=>ringedPast(p,load.result)&&(p.part.kind==='rod-span'||p.part.kind==='rod')).map((p)=>p.part.kind==='rod-span'?<g key={JSON.stringify(p.part)} data-load-part={`span:${p.part.side}:${p.part.from}:${p.part.to}`}><circle className="look" r={D.ringRadiusPx}/></g>:p.part.kind==='rod'?<g key={JSON.stringify(p.part)} data-load-part={`rod:${p.part.side}`}><circle className="look" r={D.ringRadiusPx}/></g>:null)}
      {loaded&&load.result!==null&&labels.map((label)=><g key={label.id} data-range={label.id} className="load-label-group"><path className="load-leader"/><foreignObject width="208" height="104"><div className="load-label"><p><LevelWords>{label.anchor===undefined?ESTIMATE_NAMES[label.measure]:L.anchorRange(label.level,filmSide(layout.anchors[label.anchor]?.side??'left',accepted.input.front.marks.namedFor))}</LevelWords></p><div className="num">{rangeWords(label.measure,label.band)}</div><RangeBar band={label.band} scale={load.result===null?undefined:scaleFor(load.result,label.measure)}/><small>{ESTIMATE_TAGS[label.tag]}</small></div></foreignObject></g>)}
      {([['upper', layout.upper], ['lower', layout.lower]] as const).map(([end, level]) => <g key={end} data-end={end}><text x="24" className="num">{level}</text>{result?.firstIsSketch !== true && result?.cannotTell.some((r) => r.code === 'ends-unsettled' && r.levels.includes(level)) === true && <circle className="look" r={D.ringRadiusPx} />}</g>)}
      {state.input!==null&&result!==null&&<EndHandles layout={layout} result={result} handle={handle} drawn={drawnRef}/>}
    </svg>}
    {title !== null && !state.arriving && !state.sheet && <div ref={chip} className="place-chip glass" role="status">
      <div><LevelWords>{title}</LevelWords></div>
      {anchor?.diameterMm != null && anchor.lengthMm !== null && (entered||origin!==null) && <div className="num">{W.size(anchor.diameterMm, anchor.lengthMm)}</div>}
      {rod !== undefined && <div className="num">{W.diameter(rod.diameterMm)}</div>}
      {!entered&&anchor?.kind === 'screw' && (anchor.diameterAlsoSeen.length > 0 || anchor.lengthAlsoSeen.length > 0) && <p>{W.also(anchor.diameterAlsoSeen, anchor.lengthAlsoSeen,state.ageYears)}</p>}
      {selectedLoad?.demand!=null&&<p><span className="num">{rangeWords('heaviest-anchor-demand',selectedLoad.demand)}</span> {L.strengthAllowance}</p>}
      {selectedLoad?.demand!=null&&<small>{ESTIMATE_TAGS.assumed}</small>}
      {selectedLoad!==undefined&&load.result!==null&&pastWords(selectedLoad,load.result)!==null&&<><p className="past-strength"><i className="dot" aria-hidden="true"/>{pastWords(selectedLoad,load.result)}</p><small className="load-context">{L.pastContext(selectedLoad.part.kind==='anchor')}</small><p>{L.partAssumed}</p></>}
      {loaded&&stop!=='before'&&rod?.targetKyphosisStatus==='typical'&&<p>{L.typicalRod}</p>}
      {wider !== undefined && wider.versions > 0 && result !== null && <p>{W.wider(wider.versions, result.versions)}</p>}
      {entered?<p>{importedEditWords.setByHand}</p>:anchor !== undefined && width != null && origin!==null && <p><LevelWords>{W.pedicle(width.p10, width.p90)+origin}</LevelWords></p>}
      <SourceLinks uses={[...sourceUses([...(anchor?.sources??[]),...(anchor?.pedicle.sources??[]),...(rod?.sources??[]),...(chosenEnd===undefined||result===null?[]:endSourceKeys(result,selected==='upper'?'upper':'lower'))],'layout'),...sourceUses(selectedLoad?.sources??[],'bench')]}/>
      {state.pressed!==null&&layout!==undefined&&result!==null&&state.input!==null&&<PartControls layout={layout} anchor={anchor} rod={rod} choices={result.choices}/>}
    </div>}
  </div>;
}

function EndKeyboard({selected,layout}:{selected:string|null;layout:import('@/shapes').ImplantLayout}) {
  const engine=useEngine();
  useKey('popover',['ArrowUp','ArrowDown'],(event)=>{
    if(!event.shiftKey||selected!=='upper'&&selected!=='lower')return false;
    if(handState.getState().busy!==null||loadState.getState().phase==='running')return true;
    const levels=fittedLevels(),at=levels.indexOf(layout[selected]),wanted=levels[at+(event.key==='ArrowUp'?-1:1)];
    if(wanted!==undefined){const level=snapEnd(layout,selected,wanted,levels);if(level!==layout[selected])void changePlacement(engine,{kind:'end',end:selected,level});}
    return true;
  },()=>!placeState.getState().sheet&&document.activeElement?.classList.contains('three-stage')===true);
  return null;
}
