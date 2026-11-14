import { act, useRef } from 'react';
import { Group } from 'three';
import { afterEach, expect, it, vi } from 'vitest';
import { putOnThePage, pressKey } from '@/building-blocks/test-rig/on-the-page';
import { smallStory } from '@/engine-room/stand-in/small-story';
import { MADE_UP_CASE } from '@/spine-3d/made-up-case/made-up-case';
import { sceneLayer } from '@/spine-3d/scene/scene-layer';
import type { Spine3DHandle } from '@/spine-3d/view/Spine3D';
import type { AcceptedSpine } from '@/view-state/spine-build';
import type { BenchResult } from '@/shapes';
import { loadState } from './load-state';
import { PLACE_LOAD_WORDS as L } from '@/words/place';
import { PlaceSheet } from './PlaceSheet';
import { PlaceDrawing } from './PlaceDrawing';
import { placeState } from './place-state';
import { placementFixture } from './placement-fixture.scenes';
const loop = vi.hoisted(() => new Set<(dt: number) => void>());
vi.mock('@/choreography/one-loop', () => ({ joinLoop: (step: (dt: number) => void) => { loop.add(step); return () => { loop.delete(step); }; } }));
vi.mock('@/choreography/timing', () => ({ durationOf: () => 160, curveOf: () => (p:number)=>p }));
vi.mock('./PlacementCaption', () => ({ PlacementCaption: () => null }));
afterEach(() => { vi.unstubAllGlobals(); loop.clear(); });
function mount() {
  vi.stubGlobal('getComputedStyle', () => ({ getPropertyValue: (key: string) => (({'--chip-dx':'14','--chip-dy':'16','--space-8':'8'} as Record<string,string>)[key] ?? '') }));
  vi.stubGlobal('matchMedia', () => ({ matches: false }));
  const result = placementFixture(); placeState.setState({key:'test',phase:'ready',result,arriving:false,pressed:'anchor:0',hovered:null,review:null,sheet:false,chosen:'back'});
  const root = new Group(), levels = MADE_UP_CASE.fit.vertebrae.map((v)=>v.level), holders = new Map(levels.map((level)=>{const group=new Group();root.add(group);return [level,group] as const;}));
  const listeners = new Set<()=>void>(), point = {x:100,y:200}, look = vi.fn();
  const spine = { levels, setSpine:vi.fn(), bounds:()=>({low:[0,0,0],high:[100,500,100]}), betweenPoses:vi.fn(), centre:()=>[0,0,0], set:vi.fn(), valuesOf:()=>({strength:.55}), addLayer:()=>sceneLayer(root,holders,()=>undefined,()=>undefined).layer, landmarks:()=>null };
  const handle = {spine:()=>spine,stage:()=>({view:()=>({azimuth:146,elevation:14,perspective:1,pxPerMm:[1,1],aim:[0,0,0],aimAt:[500,350]}),size:()=>({width:1000,height:700}),bounds:()=>({low:[0,0,0],high:[100,500,100]}),onDrawn:(fn:()=>void)=>{listeners.add(fn);return ()=>listeners.delete(fn);},patientToCanvas:()=>point,toCanvas:()=>point}),lookFrom:look};
  const marks = smallStory().pass.marks.find((m)=>m.film==='front'); if(marks===undefined)throw new Error('front');
  const accepted = {key:'test',fit:MADE_UP_CASE.fit,bones:[],input:{front:{marks,geometry:MADE_UP_CASE.front.geometry,size:MADE_UP_CASE.front.size,markSpread:MADE_UP_CASE.front.markSpread,facingAssumed:true},side:null,typical:'built-in',twistReadings:[]}} satisfies AcceptedSpine;
  function Drawing(){const ref=useRef(handle as unknown as Spine3DHandle);return <div tabIndex={0} className="three-stage"><PlaceDrawing accepted={accepted} handle={ref}/></div>;}
  const page=putOnThePage(<Drawing/>);
  return {page,result,point,listeners,look};
}
it('shows source words only for a pedicle with a width and keeps the other sizes to one quiet line',()=>{
  const {page,result}=mount();
  const chip=()=>page.host.querySelector('.place-chip');
  expect(chip()?.textContent).toContain('from tables for teenagers');
  expect(chip()?.querySelectorAll('p.num')).toHaveLength(0);
  expect(chip()?.textContent).toContain('Also 5.0 mm wide and 35 mm long in some versions.');
  act(()=>{placeState.setState({pressed:'rod:left'});});
  expect(chip()?.textContent).toBe('Rod, right of the front film5.5 mm');
  act(()=>{placeState.setState({pressed:'upper'});}); expect(chip()?.textContent).not.toContain('tables');
  const anchor=result.layouts[0]?.anchors[0];if(anchor===undefined)throw new Error('anchor');anchor.pedicle.outerWidthMm=null;
  act(()=>{placeState.setState({pressed:'anchor:0'});}); expect(chip()?.textContent).not.toContain('from tables');
  page.takeDown();
});
it('positions a remounted chip after the sheet closes and follows subsequent draws',()=>{
  const {page,point,listeners}=mount();
  expect(page.host.querySelector<HTMLElement>('.place-chip')?.style.left).toBe('114px');
  act(()=>{placeState.setState({sheet:true});}); expect(page.host.querySelector('.place-chip')).toBeNull();
  act(()=>{placeState.setState({sheet:false});}); expect(page.host.querySelector<HTMLElement>('.place-chip')?.style.top).toBe('216px');
  point.x=250;point.y=300;act(()=>{listeners.forEach((fn)=>{fn();});});
  expect(page.host.querySelector<HTMLElement>('.place-chip')?.style.left).toBe('264px');
  page.takeDown();
});
it('crosses rods sideways and preserves a pressed part when there is no opposite rod',()=>{
  const {page,result}=mount();
  page.host.querySelector<HTMLElement>('.three-stage')?.focus();
  act(()=>{placeState.setState({pressed:'rod:left'});}); pressKey('ArrowRight');
  expect(placeState.getState()).toMatchObject({hovered:'rod:right',pressed:null});
  const layout=result.layouts[0];if(layout===undefined)throw new Error('layout');layout.rods=layout.rods.filter((r)=>r.side==='left');
  act(()=>{placeState.setState({result:{...result,layouts:[{...layout}]}});});
  act(()=>{placeState.setState({pressed:'rod:left',hovered:null});});
  pressKey('ArrowLeft');expect(placeState.getState().pressed).toBe('rod:left');page.takeDown();
});

it('leaves M idle without a placement instead of opening a hidden sheet',()=>{
  placeState.setState({phase:'working',result:null,arriving:false,sheet:false});
  const page=putOnThePage(<PlaceSheet accepted={{} as never}/>);
  pressKey('m');expect(placeState.getState().sheet).toBe(false);
  act(()=>{placeState.setState({phase:'failed'});});pressKey('m');expect(placeState.getState().sheet).toBe(false);
  page.takeDown();
});

it('tags every anchor demand as assumed and explains past-allowance counts',()=>{
  const {page,result}=mount(),anchor=result.layouts[0]?.anchors[0];if(anchor===undefined)throw new Error('anchor');
  const part={part:{kind:'anchor' as const,level:anchor.level,side:anchor.side},share:{p10:.1,p50:.2,p90:.3},demand:{p10:.2,p50:.3,p90:.4},steady:false,versionsPastStrength:0,sources:[]};
  const answer:BenchResult={versions:32,firstId:'first',mainCurve:null,scales:[],layouts:[{layoutId:'first',versionsUnsettled:0,counts:[],stops:[{stop:'rods-in',parts:[part],estimates:[],middle:null,rods:null,ghosts:null,cyclesPerYear:null,cyclesSources:[],outsideStiffRod:0}]}]};
  act(()=>{loadState.setState({phase:'loaded',result:answer,stop:'rods-in'});});
  expect(page.host.querySelector('.place-chip > small')?.textContent).toBe('Assumed');expect(page.host.querySelector('.past-strength')).toBeNull();
  part.versionsPastStrength=1;act(()=>{loadState.setState({result:{...answer}});});
  expect(page.host.querySelector('.load-context')?.textContent).toBe(L.pastContext(true));
  expect(L.pastContext(false)).toBe('Its level is a published test strength. The count is over versions of what is not known, not a verdict on this placement.');
  page.takeDown();
});

it('hides only stage labels whose boxes overlap the open chip and restores them after it closes',()=>{
  const {page,listeners}=mount(),svg=page.host.querySelector('.place-labels'),chip=page.host.querySelector('.place-chip');
  if(svg===null||chip===null)throw new Error('drawing');
  const label=document.createElementNS('http://www.w3.org/2000/svg','g'),box=document.createElementNS('http://www.w3.org/2000/svg','foreignObject');
  label.dataset.range='test';label.append(box);svg.append(label);
  vi.spyOn(chip,'getBoundingClientRect').mockReturnValue(new DOMRect(100,100,200,200));
  const rect=vi.spyOn(box,'getBoundingClientRect').mockReturnValue(new DOMRect(200,200,208,104));
  act(()=>{listeners.forEach((fn)=>{fn();});});expect(label.style.visibility).toBe('hidden');
  rect.mockReturnValue(new DOMRect(400,100,208,104));
  act(()=>{listeners.forEach((fn)=>{fn();});});expect(label.style.visibility).toBe('');
  rect.mockReturnValue(new DOMRect(200,200,208,104));
  act(()=>{placeState.setState({pressed:null,hovered:null});});expect(label.style.visibility).toBe('');
  page.takeDown();
});
