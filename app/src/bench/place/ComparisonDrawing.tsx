import { useLayoutEffect,useMemo,useRef,useState,type RefObject } from 'react';
import { useStore } from 'zustand';
import { useKey } from '@/building-blocks/keyboard/use-key';
import { threeState } from '@/view-state/three-state';
import { LevelWords } from '@/building-blocks/LevelWords';
import { joinLoop } from '@/choreography/one-loop';
import { SYSTEM_ASKS_FOR_REDUCED } from '@/choreography/reduced-motion';
import { curveOf } from '@/choreography/timing';
import type { Spine3DHandle } from '@/spine-3d/view/Spine3D';
import type { DrawnSpine } from '@/spine-3d/scene/drawn-spine';
import type { AcceptedSpine } from '@/view-state/spine-build';
import { ESTIMATE_TAGS,PLACE_COMPARE_WORDS as W } from '@/words/place';
import { comparisonState,closeComparison } from './comparison-state';
import { comparisonName,pairedWords,comparisonDifferences,comparisonPageSize,comparisonMeasures } from './comparison-view';
import { loadState } from './load-state';
import { handState } from './hand-state';
import { placeState } from './place-state';
import { drawPlacement,PLACE_DRAWING as D,type DrawnPlacement } from './draw-placement';
import { drawingPoses,rodsInDrawing,rangeWords } from './load-view';
import { PlacementCaption } from './PlacementCaption';
import { RangeBar } from './LoadEstimates';
export const COMPARISON_DRAWING={moveMs:640,gapPx:24,footerPx:164,readingPx:156} as const;
interface ColumnDrawing {id:string;spine:DrawnSpine;hardware:DrawnPlacement}
export function ComparisonDrawing({accepted,handle}:{accepted:AcceptedSpine;handle:RefObject<Spine3DHandle|null>}) {
  const state=useStore(comparisonState),root=useRef<HTMLDivElement>(null),drawings=useRef<ColumnDrawing[]>([]),[perPage,setPerPage]=useState(comparisonPageSize(window.innerWidth)),pageSize=useRef(perPage),entered=useRef(false),progress=useRef(0);
  const measures=comparisonMeasures(loadState.getState().result),measure=measures.includes(state.measure)?state.measure:measures[0]??'heaviest-anchor-demand';
  const page=state.pageSize===perPage?state.page:0;
  const shown=useMemo(()=>[state.layouts[0],...state.layouts.slice(1+page*perPage,1+(page+1)*perPage)].filter((l)=>l!==undefined),[state.layouts,page,perPage]);
  const first=handState.getState().first?.layouts[0]??placeState.getState().result?.layouts[0];
  useKey('review',['Escape'],()=>{closeComparison();},()=>!placeState.getState().sheet);
  useLayoutEffect(()=>{
    threeState.getState().point(null);threeState.getState().light([]);
    if(comparisonState.getState().pageSize!==pageSize.current)comparisonState.setState({page:0,pageSize:pageSize.current});
    const resize=()=>{const next=comparisonPageSize(window.innerWidth);if(next!==pageSize.current){pageSize.current=next;setPerPage(next);comparisonState.setState({page:0,pageSize:next});}};
    window.addEventListener('resize',resize);return()=>{window.removeEventListener('resize',resize);};
  },[]);
  // The primary survives this screen, including a direct step change to 3D.
  useLayoutEffect(()=>{
    const stage=handle.current?.stage(),primary=handle.current?.spine();if(stage==null||primary==null)return;
    const view=stage.view(),shadow=primary.spineValues().shadow,base=drawingPoses(accepted.fit,null);
    stage.setView(()=>({...stage.namedView('three-quarter'),azimuth:D.backAzimuth,elevation:D.backElevation,perspective:1}));
    return()=>{
      stage.inColumns([],{gapPx:0,footPx:0});
      for(const drawing of drawings.current){drawing.hardware.remove();if(drawing.spine!==primary)drawing.spine.remove();}drawings.current=[];
      primary.betweenPoses(base,base,1);primary.set({strength:1});primary.setSpine({strength:1,shadow});stage.setView(view);
    };
  },[accepted,handle]);
  useLayoutEffect(()=>{
    const stage=handle.current?.stage(),primary=handle.current?.spine(),baseline=shown[0];if(stage==null||primary==null||baseline===undefined)return;
    const old=drawings.current;
    drawings.current=shown.map((layout,i)=>{
      const found=old.find((drawing)=>drawing.id===layout.id);if(found!==undefined)return found;
      const spine=i===0?primary:handle.current?.addSpine();if(spine==null)throw new Error('No fitted model for a comparison column');
      spine.set({strength:D.bone});spine.setSpine({strength:1,shadow:0});const hardware=drawPlacement(spine,accepted.fit,layout,[]);hardware.compareInk(i===0?[]:comparisonDifferences(baseline,layout));
      return {id:layout.id,spine,hardware};
    });
    for(const drawing of old)if(!drawings.current.includes(drawing)){drawing.hardware.remove();if(drawing.spine!==primary)drawing.spine.remove();}
  },[accepted,handle,shown]);
  useLayoutEffect(()=>{
    const stage=handle.current?.stage(),part=root.current;if(stage==null||part===null)return;
    const opening=!entered.current;entered.current=true;
    const reduced=window.matchMedia(SYSTEM_ASKS_FOR_REDUCED).matches||part.closest('[data-motion="reduced"]')!==null,ease=curveOf(part,'--ease-settle');let started:number|null=null;
    part.dataset['settled']='false';part.dataset['motion']=state.leaving?'leaving':opening?'entering':'page';
    const stop=joinLoop((_dt,now)=>{
      started??=now;const p=Math.min(1,(now-started)/COMPARISON_DRAWING.moveMs);
      progress.current=reduced?1:state.leaving?1-ease(p):opening?ease(p):1;
      stage.inColumns(drawings.current.map((d)=>d.spine),{gapPx:COMPARISON_DRAWING.gapPx,footPx:footer(stage.size().height,part),progress:progress.current});
      drawings.current.forEach((d,i)=>{const opacity=state.leaving?1-p:opening||i>0?p:1;d.spine.setSpine({strength:i===0&&!reduced?1:opacity});const column=part.children[i];if(column instanceof HTMLElement)column.style.opacity=String(opacity);});
      if(p===1){part.dataset['settled']='true';stop();}
    });return stop;
  },[handle,shown,state.leaving]);
  useLayoutEffect(()=>{
    const stage=handle.current?.stage();if(stage==null)return;
    const base=drawingPoses(accepted.fit,null);
    drawings.current.forEach((drawing,i)=>{const layout=shown[i];if(layout===undefined)return;const result=state.result===null?null:{...state.result,firstId:layout.id},poses=result===null?null:rodsInDrawing(result,state.batches,accepted.fit);
      drawing.spine.betweenPoses(base,poses?.poses??base,1);drawing.hardware.poses(poses?.poses??base,poses!==null&&poses.source!=='fit',poses?.rods??null);drawing.hardware.compareInk(i===0?[]:comparisonDifferences(shown[0]??layout,layout));
    });
  },[accepted,handle,shown,state.result,state.batches]);
  useLayoutEffect(()=>{
    const stage=handle.current?.stage(),part=root.current;if(stage==null||part===null)return;let geometry='';
    const project=()=>{
      const foot=footer(stage.size().height,part),next=JSON.stringify([stage.size(),foot]);
      if(next!==geometry){geometry=next;stage.inColumns(drawings.current.map((d)=>d.spine),{gapPx:COMPARISON_DRAWING.gapPx,footPx:foot,progress:progress.current});}
      drawings.current.forEach((d,i)=>{const cell=stage.columnOf(d.spine),column=part.children[i];if(cell===null||!(column instanceof HTMLElement))return;column.style.left=`${String(cell.left)}px`;column.style.width=`${String(cell.right-cell.left)}px`;column.style.top=`${String(cell.bottom+12)}px`;});
    };project();return stage.onDrawn(project);
  },[handle,shown]);
  return <><div className="place-comparison" ref={root}>{shown.map((layout,i)=>{
    const done=state.finished.some((l)=>l.id===layout.id),result=state.result?.layouts.find((l)=>l.layoutId===layout.id),count=result?.counts.find((c)=>c.measure===measure&&c.stop==='rods-in'),words=pairedWords(count),estimate=result?.stops.find((s)=>s.stop==='rods-in')?.estimates.find((e)=>e.measure===measure);
    return <section className="place-column" key={layout.id} data-layout={layout.id} data-finished={done} aria-label={first===undefined?'':comparisonName(layout,first)}>
      <h3><LevelWords>{first===undefined?'':comparisonName(layout,first)}</LevelWords></h3>
      {i===0?state.phase!=='running'&&estimate!==undefined&&<><p className="num">{rangeWords(measure,estimate.band)}</p><RangeBar band={estimate.band} scale={state.result?.scales.find((s)=>s.measure===measure)}/><small>{ESTIMATE_TAGS[estimate.tag]}</small></>:done?<><div className="comparison-dots" aria-hidden="true">{words.dots.map((outcome,j)=><i key={count?.order[j]??j} data-outcome={outcome}/>)}</div><p>{words.verdict}</p>{words.clear!==''&&<small>{words.clear}</small>}</>:<p>{W.working}</p>}
    </section>;
  })}</div><PlacementCaption/></>;
}

function footer(height:number,part:HTMLElement):number {
  const dock=document.querySelector('.dock')?.getBoundingClientRect();return dock===undefined?COMPARISON_DRAWING.footerPx:Math.max(COMPARISON_DRAWING.footerPx,height-(dock.top-part.getBoundingClientRect().top)+COMPARISON_DRAWING.readingPx);
}
