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
import { ruleCounts } from './end-rules';
export function EndHandles({layout,result,handle,drawn}:{layout:ImplantLayout;result:LayoutProposals;handle:RefObject<Spine3DHandle|null>;drawn:RefObject<DrawnPlacement|null>}) {
  const engine=useEngine(),state=useStore(handState),root=useRef<SVGGElement>(null),levels=fittedLevels(),original=state.first??result;
  useKey('popover',['Escape'],cancelEndDrag,()=>handState.getState().drag!==null);
  useLayoutEffect(()=>{
    const end=state.drag?.end,part=drawn.current;if(end===undefined||part===null)return;part.fadeParts([end],0);return ()=>{part.fadeParts([end],1);};
  },[drawn,state.drag?.end]);
  useLayoutEffect(()=>{
    const stage=handle.current?.stage(),spine=handle.current?.spine(),surface=root.current;if(stage==null||spine==null||surface===null)return;
    const project=()=>{
      for(const node of surface.querySelectorAll<SVGGElement>('[data-end-handle], [data-rule-level]')){
        const end=node.dataset.endHandle as End|undefined,level=end===undefined?node.dataset.ruleLevel as Level:state.drag?.end===end?state.drag.level:layout[end];
        const mark=spine.landmarks(level);if(mark===null)continue;
        const plate=end==='lower'||end===undefined&&state.drag?.end==='lower'?mark.bottom:mark.top,p=stage.toCanvas(plate.centre);
        node.setAttribute('transform',`translate(${String(p.x)} ${String(p.y)})`);
        const left=stage.toCanvas(plate.left),right=stage.toCanvas(plate.right),css=getComputedStyle(surface);
        const preview=node.querySelector('line');if(preview!==null){preview.setAttribute('x1',String(left.x-p.x));preview.setAttribute('y1',String(left.y-p.y));preview.setAttribute('x2',String(right.x-p.x));preview.setAttribute('y2',String(right.y-p.y));}
        node.querySelector('text')?.setAttribute('x',String(end===undefined?Math.min(left.x,right.x)-p.x-parseFloat(css.getPropertyValue('--space-16')):Math.max(left.x,right.x)-p.x+parseFloat(css.getPropertyValue('--space-8'))));
      }
    };project();return stage.onDrawn(project);
  },[handle,layout,state.drag,levels]);
  const move=(clientX:number,clientY:number)=>{
    const drag=handState.getState().drag,stage=handle.current?.stage(),spine=handle.current?.spine(),box=root.current?.ownerSVGElement?.getBoundingClientRect();if(drag===null||stage==null||spine==null||box===undefined)return;
    let wanted=drag.level,distance=Infinity;
    for(const level of levels){const mark=spine.landmarks(level);if(mark===null)continue;const p=stage.toCanvas((drag.end==='upper'?mark.top:mark.bottom).centre),d=Math.hypot(p.x-(clientX-box.left),p.y-(clientY-box.top));if(d<distance){distance=d;wanted=level;}}
    handState.setState({drag:{end:drag.end,level:snapEnd(layout,drag.end,wanted,levels)}});
  };
  const counts=state.drag===null?[]:ruleCounts(original,state.drag.end),total=counts[0]?.total;
  return <g ref={root} className="place-end-handles">
    {state.drag!==null&&total!==undefined&&endLevels(layout,levels,state.drag.end).filter((level)=>endReason(layout,state.drag?.end??'upper',level)===null).map((level)=><g key={level} data-rule-level={level}><text textAnchor="end">{W.rules(counts.find((r)=>r.level===level)?.count??0,total)}</text></g>)}
    {(['upper','lower'] as const).map((end)=><g key={end} data-end-handle={end} data-held={state.drag?.end===end||undefined}>{state.drag?.end===end&&<line className="end-preview"/>}<circle r={D.endHandleRadiusPx} onPointerDown={(event)=>{if(state.busy!==null)return;event.preventDefault();event.stopPropagation();event.currentTarget.setPointerCapture(event.pointerId);placeState.setState({pressed:null,hovered:null,keyboard:false});handState.setState({drag:{end,level:layout[end]}});}} onPointerMove={(event)=>{if(handState.getState().drag?.end===end){event.preventDefault();event.stopPropagation();move(event.clientX,event.clientY);}}} onPointerUp={(event)=>{const drag=handState.getState().drag;if(drag===null)return;event.preventDefault();event.stopPropagation();event.currentTarget.releasePointerCapture(event.pointerId);handState.setState({drag:null});if(drag.level!==layout[end])void changePlacement(engine,{kind:'end',end,level:drag.level});else openWhy(end);}} onPointerCancel={()=>{handState.setState({drag:null});}}/><text className="num">{state.drag?.end===end?state.drag.level:layout[end]}</text></g>)}
  </g>;
}
