import { useLayoutEffect, useRef, type RefObject } from 'react';
import { useStore } from 'zustand';
import type { Spine3DHandle } from '@/spine-3d/view/Spine3D';
import type { AcceptedSpine } from '@/view-state/spine-build';
import { joinLoop } from '@/choreography/one-loop';
import { curveOf, durationOf } from '@/choreography/timing';
import { SYSTEM_ASKS_FOR_REDUCED } from '@/choreography/reduced-motion';
import { poseBetween } from '@/spine-3d/placing/pose-between';
import { FORMATION } from '@/spine-3d/scene/look';
import type { DrawnSpine } from '@/spine-3d/scene/drawn-spine';
import type { DrawnPlacement } from './draw-placement';
import { loadState } from './load-state';
import { drawingPoses, rodsInDrawing, stopResult, shownLoadStop, shownStopResult,tooFewSettled } from './load-view';
export const LOAD_DRAWING = { moveMs: 600, ghostStrength: .12, labelGapPx: 24, rangeWidthPx: 64 } as const;
/** Drives only drawing poses. It never changes the fit or any estimate. */
export function useLoadDrawing(accepted: AcceptedSpine, handle: RefObject<Spine3DHandle|null>, drawn: RefObject<DrawnPlacement|null>, surface: RefObject<HTMLElement|null>) {
  const load=useStore(loadState), poses=useRef(drawingPoses(accepted.fit,null)), ghosts=useRef<DrawnSpine[]>([]), moving=useRef(false);
  useLayoutEffect(()=>{
    const stage=handle.current?.stage(),spine=handle.current?.spine();
    if(stage==null||spine==null||load.result===null||tooFewSettled(load.result))return;
    ghosts.current=(stopResult(load.result,'rods-in')?.ghosts??[]).slice(0,2).flatMap((at)=>{
      const ghost=handle.current?.addFlatSpine();if(ghost==null)return[];ghost.setSpine({flatColour:FORMATION.ink,strength:0,shadow:0});ghost.betweenPoses(drawingPoses(accepted.fit,null),drawingPoses(accepted.fit,at),1);return [ghost];
    });
    return ()=>{for(const ghost of ghosts.current)ghost.remove();ghosts.current=[];};
  },[accepted,handle,load.result]);
  useLayoutEffect(()=>{
    const spine=handle.current?.spine(),stage=handle.current?.stage(),part=surface.current,hardware=drawn.current;
    if(spine==null||part==null||hardware===null)return;
    const base=drawingPoses(accepted.fit,null),loaded=load.phase==='loaded'&&load.result!==null;
    const drawing=load.result===null?null:rodsInDrawing(load.result,load.batches,accepted.fit);
    const before=!loaded||shownLoadStop(load.stop)==='before',target=before?base:drawing?.poses??base,from=poses.current;
    const shapeChange=from.some((p,i)=>JSON.stringify(p.centre)!==JSON.stringify(target[i]?.centre)||JSON.stringify(p.rotation)!==JSON.stringify(target[i]?.rotation));
    const reduced=window.matchMedia(SYSTEM_ASKS_FOR_REDUCED).matches||part.closest('[data-motion="reduced"]')!==null;
    const fade=durationOf(part,'--dur-appear'),travel=reduced?0:shapeChange?LOAD_DRAWING.moveMs:0,curve=curveOf(part,'--ease-settle');
    const parts=loaded&&!before&&!tooFewSettled(load.result)?shownStopResult(load.result,load.stop)?.parts??[]:null;
    // Create seated spans before taking their current ink strengths.
    hardware.poses(from,loaded&&!before&&drawing?.source!=='fit',drawing?.rods??null,drawing?.poses,before&&shapeChange);
    const ink=hardware.loadTo(parts),ghostFrom=ghosts.current.map((g)=>g.spineValues().strength);
    let elapsed=0, faded=false;moving.current=shapeChange;
    const frame=(dt:number)=>{
      elapsed+=dt;
      const fadeProgress=fade===0?1:Math.min(1,elapsed/fade);
      const p=reduced&&shapeChange?(fadeProgress<.5?0:1):travel===0?1:curve(Math.min(1,elapsed/travel));
      if(reduced&&shapeChange)spine.setSpine({strength:Math.abs(1-2*fadeProgress)});
      poses.current=target.map((to,i)=>poseBetween(from[i]??to,to,p));
      spine.betweenPoses(base,poses.current,1);
      if(shapeChange)hardware.poses(poses.current,loaded&&!before&&drawing?.source!=='fit',drawing?.rods??null,drawing?.poses,before&&p<1);
      if(!reduced||!shapeChange||faded)ink(fade===0?1:Math.min(1,elapsed/fade));
      ghosts.current.forEach((ghost,i)=>{
        const to=before?0:LOAD_DRAWING.ghostStrength,progress=fade===0?1:Math.max(0,Math.min(1,(elapsed-(before?0:travel))/fade));
        ghost.setSpine({strength:(ghostFrom[i]??0)+(to-(ghostFrom[i]??0))*progress});
      });
      // Restore whole-spine opacity before applying the final local part ink.
      if(reduced&&shapeChange&&fadeProgress===1&&!faded){faded=true;return false;}
      if(elapsed>=travel+fade){moving.current=false;return true;}return false;
    };
    const stop=joinLoop((dt)=>{if(frame(dt))stop();});
    return ()=>{stop();moving.current=false;spine.setSpine({strength:1});if(reduced)stage?.drawNow();};
  },[accepted,drawn,handle,load.batches,load.phase,load.result,load.stop,surface]);
  useLayoutEffect(()=>()=>{const spine=handle.current?.spine();const base=drawingPoses(accepted.fit,null);spine?.betweenPoses(base,base,1);},[accepted,handle]);
  return {poses,moving};
}
