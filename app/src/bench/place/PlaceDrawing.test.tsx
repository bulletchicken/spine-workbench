import { act, useRef } from 'react';
import { Group } from 'three';
import { afterEach, expect, it, vi } from 'vitest';
import { putOnThePage, pressKey } from '@/building-blocks/test-rig/on-the-page';
import { smallStory } from '@/engine-room/stand-in/small-story';
import { MADE_UP_CASE } from '@/spine-3d/made-up-case/made-up-case';
import { sceneLayer } from '@/spine-3d/scene/scene-layer';
import type { Spine3DHandle } from '@/spine-3d/view/Spine3D';
import type { AcceptedSpine } from '@/view-state/spine-build';
import type { BenchResult, PartResult } from '@/shapes';
import { loadState } from './load-state';
import { PLACE_LOAD_WORDS as L } from '@/words/place';
import { LoadEstimates } from './LoadEstimates';
import { PlaceSheet } from './PlaceSheet';
import { PlaceDrawing } from './PlaceDrawing';
import { handState, openWhy } from './hand-state';
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
  return {page,result,point,listeners,look,accepted};
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
  expect(page.host.querySelector('.past-strength > .dot')?.getAttribute('aria-hidden')).toBe('true');
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

function bench(parts:PartResult[]=[]):BenchResult {
  return {versions:32,firstId:'first',mainCurve:null,scales:[],layouts:[{layoutId:'first',versionsUnsettled:0,counts:[],stops:[{stop:'rods-in',parts,estimates:[],middle:null,rods:null,ghosts:null,cyclesPerYear:null,cyclesSources:[],outsideStiffRod:0}]}]};
}
it('uses mono only for levels in review labels and past-allowance part names, with a look dot in the row',()=>{
  const {page,result,accepted}=mount();page.takeDown();
  const a=result.layouts[0]?.anchors[0];if(a===undefined)throw new Error('anchor');
  act(()=>{loadState.setState({phase:'loaded',stop:'rods-in',result:bench([{part:{kind:'anchor',level:a.level,side:a.side},share:{p10:.1,p50:.2,p90:.3},demand:null,steady:false,versionsPastStrength:1,sources:[]}])});placeState.setState({sheet:true});});
  const sheet=putOnThePage(<PlaceSheet accepted={accepted}/>),labels=[...sheet.host.querySelectorAll('.place-review-label')];
  expect(labels.map((n)=>n.textContent)).toContain('Every pedicle');expect(labels.map((n)=>n.textContent)).toContain('Level names');
  expect(labels.some((n)=>n.textContent==='Upper end · T4'&&n.querySelector('.level-name')?.textContent==='T4')).toBe(true);
  expect(labels.every((n)=>!n.classList.contains('num')&&[...n.querySelectorAll('.level-name')].every((level)=>/^(T\d+|L\d+)$/.test(level.textContent)))).toBe(true);
  const past=sheet.host.querySelector('.load-estimates > .past-strength');expect(past?.querySelector('.dot')?.getAttribute('aria-hidden')).toBe('true');
  expect(past?.querySelector('.level-name')?.textContent).toBe(a.level);expect(past?.querySelector('.num')).toBeNull();
  sheet.takeDown();
});
it('shows the typical contour note only for that rod and span after Before',()=>{
  const {page,result}=mount(),left=result.layouts[0]?.rods.find((r)=>r.side==='left');if(left===undefined)throw new Error('rod');left.targetKyphosisStatus='typical';
  act(()=>{loadState.setState({phase:'loaded',stop:'rods-in',result:bench()});placeState.setState({pressed:'rod:right'});});
  expect(page.host.querySelector('.place-chip')?.textContent).not.toContain(L.typicalRod);
  act(()=>{placeState.setState({pressed:'rod:left'});});expect(page.host.querySelector('.place-chip')?.textContent).toContain(L.typicalRod);
  act(()=>{placeState.setState({pressed:'span:left:T4:T5'});});expect(page.host.querySelector('.place-chip')?.textContent).toContain(L.typicalRod);
  act(()=>{loadState.setState({stop:'before'});});expect(page.host.querySelector('.place-chip')?.textContent).not.toContain(L.typicalRod);
  page.takeDown();
});
it('leaves daily-only variation lines out of the sheet while retaining other sources',()=>{
  const {page,accepted}=mount();page.takeDown();const answer=bench(),stop=answer.layouts[0]?.stops[0];if(stop===undefined)throw new Error('stop');
  const unknowns=['weight','steps','walking','contour-knock-down','geometry'] as const;
  const measures=['heaviest-anchor-force','heaviest-anchor-demand','heaviest-anchor-over-median','heaviest-rod-use','junction-above'] as const;
  stop.estimates=unknowns.map((mostlyFrom,i)=>({measure:measures[i]??'junction-above',band:{p10:i,p50:i+1,p90:i+2},tag:'assumed',mostlyFrom,sources:[]}));
  act(()=>{loadState.setState({phase:'loaded',result:answer,stop:'rods-in'});});
  const sheet=putOnThePage(<LoadEstimates accepted={accepted}/>);
  expect([...sheet.host.querySelectorAll('.load-estimate-list li > p')].map((p)=>p.textContent)).toEqual(['Varies most with: the fit.']);
  sheet.takeDown();
});

it('keeps stage focus when Escape closes a kept chip before walking again',()=>{
  const {page}=mount(),stage=page.host.querySelector<HTMLElement>('.three-stage');stage?.focus();pressKey('Escape');
  expect(placeState.getState()).toMatchObject({pressed:null,keyboard:true});expect(document.activeElement).toBe(stage);
  pressKey('ArrowDown');expect(placeState.getState().hovered).toBe('upper');page.takeDown();
});

it('uses only entered engine sizes for the source marker, restoring table words and alternatives',()=>{
  const {page,result}=mount(),anchor=result.layouts[0]?.anchors[0];if(anchor===undefined)throw new Error('anchor');
  act(()=>{handState.setState({choices:[{level:anchor.level,side:anchor.side,kind:'screw',diameterMm:anchor.diameterMm,lengthMm:anchor.lengthMm}]});placeState.setState({result:{...result}});});
  expect(page.host.querySelector('.place-chip')?.textContent).not.toContain('Set by hand');expect(page.host.querySelector('.place-chip')?.textContent).toContain('Also ');
  anchor.diameterStatus='entered';act(()=>{placeState.setState({result:{...result}});});expect(page.host.querySelector('.place-chip')?.textContent).toContain('Set by hand');
  anchor.diameterStatus='from-tables';anchor.lengthStatus='from-tables';act(()=>{placeState.setState({result:{...result}});});
  expect(page.host.querySelector('.place-chip')?.textContent).toContain('from tables for teenagers');expect(page.host.querySelector('.place-chip')?.textContent).toContain('Also ');page.takeDown();
});
it('resets Why here through Escape, Done, outside press and M, including reopening with M',()=>{
  const {page,accepted}=mount();page.takeDown();const sheet=putOnThePage(<PlaceSheet accepted={accepted}/>);
  for(const close of [()=>pressKey('Escape'),()=>{const done=[...sheet.host.querySelectorAll('button')].find((b)=>b.textContent==='Done');act(()=>{done?.click();});},()=>{act(()=>{document.body.dispatchEvent(new MouseEvent('pointerdown',{bubbles:true}));});},()=>pressKey('m')]){
    act(()=>{openWhy('lower');});expect(sheet.host.querySelector('#place-sheet-title')?.textContent).toBe('Why here');close();expect(handState.getState().why).toBeNull();
    pressKey('m');expect(sheet.host.querySelector('#place-sheet-title')?.textContent).toBe('What needs a look');pressKey('Escape');
  }
  sheet.takeDown();
});

it('gives the daily and beside-rods notes once without expecting missing adjacent-level estimates',()=>{
  const {page,accepted,result}=mount();page.takeDown();const layout=result.layouts[0];if(layout===undefined)throw new Error('layout');layout.upper='T1';
  act(()=>{loadState.setState({phase:'loaded',result:bench(),stop:'rods-in'});});
  const sheet=putOnThePage(<LoadEstimates accepted={accepted}/>),notes=[...sheet.host.querySelectorAll('.panel-note p')].map((p)=>p.textContent);
  expect(notes.filter((line)=>line===L.dailyLoads)).toHaveLength(1);expect(notes.filter((line)=>line===L.besideRods)).toHaveLength(1);
  expect(sheet.host.textContent).not.toContain('Starts at T1: there is no level above');expect(sheet.host.textContent).not.toContain('Loads in daily life are not worked out yet.');
  expect([...sheet.host.querySelectorAll('.load-estimate-list li > span')].map((n)=>n.textContent)).not.toContain('Level above');sheet.takeDown();
});
