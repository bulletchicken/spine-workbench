import { useLayoutEffect, useRef, type RefObject } from 'react';
import { useStore } from 'zustand';
import { type ImplantLayout, type LayoutProposals, type Level } from '@/shapes';
import type { Spine3DHandle } from '@/spine-3d/view/Spine3D';
import { useKey } from '@/building-blocks/keyboard/use-key';
import { useEngine } from '@/engine-room/use-engine';
import { PLACE_EDIT_WORDS as W } from '@/words/place';
import { cancelEndDrag, changePlacement, endLevels, endReason, fittedLevels, handState, openWhy, snapEnd, type End } from './hand-state';
import type { DrawnPlacement } from './draw-placement';
import { PLACE_DRAWING as D } from './draw-placement';
import { placeState } from './place-state';
import { loadState } from './load-state';
import { ruleCounts } from './end-rules';
type Stage=NonNullable<ReturnType<Spine3DHandle['stage']>>;
/**
 * An end's level name sits `gap` beyond the endplate's outer rim, and `gap` beyond any of the hardware it would otherwise lie on, after
 * clearing a ring's radius or a head's, whichever is larger. Hardware counts when it is drawn within that clearance and two gaps of the
 * name's line, a band that holds the name's letters: a drawing allowance of this project, not a measurement.
 */
export function endTextX(stage:Stage,spine:NonNullable<ReturnType<Spine3DHandle['spine']>>,rim:readonly number[],y:number,reach:readonly (readonly [number,number,number])[],headRadiusMm:number,gap:number):number {
  const clear=Math.max(D.ringRadiusPx,headRadiusMm*stage.view().pxPerMm[0]);
  return Math.max(...rim.map((x)=>x+gap),...reach.map((p)=>stage.patientToCanvas([p[0],p[1],p[2]],spine)).filter((q)=>Math.abs(q.y-y)<=clear+2*gap).map((q)=>q.x+clear+gap));
}
export function EndHandles({layout,result,handle,drawn}:{layout:ImplantLayout;result:LayoutProposals;handle:RefObject<Spine3DHandle|null>;drawn:RefObject<DrawnPlacement|null>}) {
  const engine=useEngine(),state=useStore(handState),root=useRef<SVGGElement>(null),levels=fittedLevels(),original=state.first??result;
  useKey('popover',['Escape'],cancelEndDrag,()=>handState.getState().drag!==null);
  useLayoutEffect(()=>{
    const end=state.drag?.end,part=drawn.current;if(end===undefined||part===null)return;part.fadeParts([end],0);return ()=>{part.fadeParts([end],1);};
  },[drawn,state.drag?.end]);
  useLayoutEffect(()=>{
    const stage=handle.current?.stage(),spine=handle.current?.spine(),surface=root.current;if(stage==null||spine==null||surface===null)return;
    // The page's spacing, read once and again when the page changes size; nothing is moved when neither the view nor the spine has.
    let small=0,large=0,last='',was:unknown=null,wasDrawn:unknown=null;const placed=new WeakSet<Element>();
    const readStyle=()=>{const css=getComputedStyle(surface);small=parseFloat(css.getPropertyValue('--space-8'));large=parseFloat(css.getPropertyValue('--space-16'));last='';};
    readStyle();const resized=new ResizeObserver(readStyle),sheet=surface.ownerSVGElement;if(sheet!==null)resized.observe(sheet);
    const project=()=>{
      const view=stage.view(),size=stage.size(),bounds=spine.bounds();
      const key=[view.azimuth,view.elevation,view.perspective,...view.pxPerMm,...view.aim,...view.aimAt,size.width,size.height].join();
      const nodes=surface.querySelectorAll<SVGGElement>('[data-end-handle], [data-rule-level]');
      if(key===last&&bounds===was&&drawn.current===wasDrawn&&[...nodes].every((node)=>placed.has(node)))return;last=key;was=bounds;wasDrawn=drawn.current;
      for(const node of nodes){placed.add(node);
        const end=node.dataset.endHandle as End|undefined,level=end===undefined?node.dataset.ruleLevel as Level:state.drag?.end===end?state.drag.level:layout[end];
        const mark=spine.landmarks(level);if(mark===null)continue;
        const plate=end==='lower'||end===undefined&&state.drag?.end==='lower'?mark.bottom:mark.top,p=stage.toCanvas(plate.centre);
        node.setAttribute('transform',`translate(${String(p.x)} ${String(p.y)})`);
        const left=stage.toCanvas(plate.left),right=stage.toCanvas(plate.right);
        const preview=node.querySelector('line');if(preview!==null){preview.setAttribute('x1',String(left.x-p.x));preview.setAttribute('y1',String(left.y-p.y));preview.setAttribute('x2',String(right.x-p.x));preview.setAttribute('y2',String(right.y-p.y));}
        node.querySelector('text')?.setAttribute('x',String(end===undefined?Math.min(left.x,right.x)-p.x-large:endTextX(stage,spine,[left.x,right.x],p.y,level===layout[end]?drawn.current?.reach(end)??[]:[],(layout.rods[0]?.diameterMm??0)*D.headOverRod/2,small)-p.x));
      }
    };project();const unstage=stage.onDrawn(project);return ()=>{unstage();resized.disconnect();};
  },[drawn,handle,layout,state.drag,levels]);
  const move=(clientX:number,clientY:number)=>{
    const drag=handState.getState().drag,stage=handle.current?.stage(),spine=handle.current?.spine(),box=root.current?.ownerSVGElement?.getBoundingClientRect();if(drag===null||stage==null||spine==null||box===undefined)return;
    let wanted=drag.level,distance=Infinity;
    for(const level of levels){const mark=spine.landmarks(level);if(mark===null)continue;const p=stage.toCanvas((drag.end==='upper'?mark.top:mark.bottom).centre),d=Math.hypot(p.x-(clientX-box.left),p.y-(clientY-box.top));if(d<distance){distance=d;wanted=level;}}
    handState.setState({drag:{end:drag.end,level:snapEnd(layout,drag.end,wanted,levels)}});
  };
  const counts=state.drag===null?[]:ruleCounts(original,state.drag.end),total=counts[0]?.total;
  return <g ref={root} className="place-end-handles">
    {state.drag!==null&&total!==undefined&&endLevels(layout,levels,state.drag.end).filter((level)=>endReason(layout,state.drag?.end??'upper',level)===null).map((level)=><g key={level} data-rule-level={level}><text textAnchor="end">{W.rules(counts.find((r)=>r.level===level)?.count??0,total)}</text></g>)}
    {(['upper','lower'] as const).map((end)=><g key={end} data-end-handle={end} data-held={state.drag?.end===end||undefined}>{state.drag?.end===end&&<line className="end-preview"/>}<circle r={D.endHandleRadiusPx} onPointerDown={(event)=>{if(state.busy!==null||loadState.getState().phase==='running')return;event.preventDefault();event.stopPropagation();event.currentTarget.setPointerCapture(event.pointerId);placeState.setState({pressed:null,hovered:null,keyboard:false});handState.setState({drag:{end,level:layout[end]}});}} onPointerMove={(event)=>{if(handState.getState().drag?.end===end){event.preventDefault();event.stopPropagation();move(event.clientX,event.clientY);}}} onPointerUp={(event)=>{const drag=handState.getState().drag;if(drag===null)return;event.preventDefault();event.stopPropagation();event.currentTarget.releasePointerCapture(event.pointerId);handState.setState({drag:null});if(drag.level!==layout[end])void changePlacement(engine,{kind:'end',end,level:drag.level});else openWhy(end);}} onPointerCancel={()=>{handState.setState({drag:null});}}/><text className="num">{state.drag?.end===end?state.drag.level:layout[end]}</text></g>)}
  </g>;
}
