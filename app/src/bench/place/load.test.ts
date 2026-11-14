import { Group, Mesh, MeshBasicMaterial } from 'three';
import { sceneLayer } from '@/spine-3d/scene/scene-layer';
import { currentStep } from '@/view-state/current-step';
import { drawPlacement } from './draw-placement';
import { placeDock } from './place-dock';
import { beforeEach, expect, it, vi } from 'vitest';
import type { BenchBatch, BenchInput, BenchCompareInput, BenchResult, BenchRun, BenchStop, Measure, PartResult, StopResult } from '@/shapes';
import { EngineFailure } from '@/engine-room/engine-client';
import { emptyCase } from '@/case-state/empty-case';
import { MADE_UP_CASE } from '@/spine-3d/made-up-case/made-up-case';
import { ESTIMATE_NAMES, LOAD_STOPS, SHOWN_LOAD_STOPS, PLACE_LOAD_WORDS as W } from '@/words/place';
import { layoutInput } from './layout-input';
import { placementFixture } from './placement-fixture.scenes';
import { limitsForPlacement } from './placement-view';
import { placeState } from './place-state';
import { loadFailure, loadPlacement, loadState, stopLoading } from './load-state';
import { anchorPoint, drawingPoses, estimatesAt, loadLabels, rangeWords, rodsInDrawing, shownLoadStop, shownStopResult, variationWords } from './load-view';
let key=0;
const band=(p50:number)=>({p10:p50-.1,p50,p90:p50+.1});
function result(): BenchResult {
  const stops:BenchStop[]=['before','rods-in','standing','bending-forward','year-of-walking'];
  const parts:PartResult[]=placementFixture().layouts[0]?.anchors.slice(0,2).map((a)=>({part:{kind:'anchor',level:a.level,side:a.side},share:band(.5),demand:band(.8),steady:false,versionsPastStrength:2,sources:[]}))??[];
  return {versions:32,firstId:'first',mainCurve:{upper:'T4',apex:'T7',lower:'L1'},scales:[],layouts:[{layoutId:'first',versionsUnsettled:3,counts:[],stops:stops.map((stop):StopResult=>({stop,estimates:[{measure:'main-cobb',band:{p10:18,p50:22,p90:26},tag:'assumed',mostlyFrom:null,sources:[]},{measure:'kyphosis',band:band(25),tag:'assumed',mostlyFrom:null,sources:[]}],parts:stop==='before'?[]:parts,middle:null,rods:null,ghosts:null,cyclesPerYear:null,cyclesSources:[],outsideStiffRod:0}))}]};
}
beforeEach(()=>{placeState.setState({key:String(key++),result:placementFixture(),arriving:false,phase:'ready'});});
it('sends every slice from zero once with the same inputs and compares only the stage placement',async()=>{
  const input={...layoutInput(MADE_UP_CASE.fit,emptyCase()),versions:10};
  const engine={runLoadBench:vi.fn<(input:BenchInput)=>Promise<BenchBatch>>().mockImplementation((s)=>Promise.resolve({versionFrom:s.versionFrom,versionCount:s.versionCount,runs:[],...{middle:[],mainCurve:null}})),compareLayouts:vi.fn<(input:BenchCompareInput)=>Promise<BenchResult>>().mockResolvedValue(result())};
  await loadPlacement(engine,input);
  expect(engine.runLoadBench.mock.calls.map(([i])=>[i.versionFrom,i.versionCount])).toEqual([[0,4],[4,4],[8,2]]);
  for(const [slice] of engine.runLoadBench.mock.calls){expect(slice).toMatchObject({fit:input.fit,child:{ageYears:null,weightKg:null,activity:null},seed:input.seed,versions:10});expect(slice.layouts).toHaveLength(1);}
  expect(engine.compareLayouts.mock.calls[0]?.[0]).toMatchObject({layouts:[placeState.getState().result?.layouts[0]],batches:[{versionFrom:0},{versionFrom:4},{versionFrom:8}]});
  expect(loadState.getState()).toMatchObject({phase:'loaded',completed:10,total:10});
});
it('Stop ignores the running slice, sends no next one, and retains no result',async()=>{
  let finish:((b:BenchBatch)=>void)|undefined;
  const engine={runLoadBench:vi.fn(()=>new Promise<BenchBatch>((resolve)=>{finish=resolve;})),compareLayouts:vi.fn()};
  const pending=loadPlacement(engine,layoutInput(MADE_UP_CASE.fit,emptyCase()));
  stopLoading();finish?.({versionFrom:0,versionCount:4,runs:[],...{middle:[],mainCurve:null}});await pending;
  expect(engine.runLoadBench).toHaveBeenCalledOnce();expect(engine.compareLayouts).not.toHaveBeenCalled();
  expect(loadState.getState()).toMatchObject({phase:'stopped',result:null,batches:[],completed:0});
});
it('chooses the settled rods-in version nearest the supplied middle and lets the separate drawing win',()=>{
  const answer=result(),input=layoutInput(MADE_UP_CASE.fit,emptyCase());
  const poses=input.fit.vertebrae.map((v)=>({...v.pose,centre:[v.pose.centre[0],v.pose.centre[1]+5,v.pose.centre[2]] as [number,number,number]}));
  const run=(version:number,mainCobbDeg:number,settled=true):BenchRun=>({layoutId:'first',version,settled,stiffRodShare:0,draws:{stiffnessScale:1,ribFactors:[1,1,1],boneFactor:1,weightKg:40,stepsPerDay:10000,walkingMultiple:2,rodYieldMpa:1000,preloadFactor:1,sliceOffsetMm:10,contourKnockDown:1},stops:[{stop:'rods-in',mainCobbDeg,kyphosisDeg:25,apicalTwistDeg:0,anchorForceN:[],anchorDemand:[],rodSpanUse:[],junctionAbove:null,junctionBelow:null,rodSwing:null,cyclesPerYear:null,poses:poses.map((p)=>({...p,centre:[p.centre[0]+version,p.centre[1],p.centre[2]]}))}]});
  const batches:BenchBatch[]=[{versionFrom:0,versionCount:3,...{middle:[],mainCurve:null},runs:[run(0,22,false),run(1,20),run(2,25)]}];
  const first=input.fit.vertebrae[0];if(first===undefined)throw new Error('vertebra');
  const extra=({...first,level:'C7',pose:{...first.pose,level:'C7'}} as const);
  expect(drawingPoses({...input.fit,vertebrae:[extra,...input.fit.vertebrae]},null).some((p)=>p.level==='C7')).toBe(false);
  const chosen=rodsInDrawing(answer,batches,input.fit);expect(chosen.source).toBe('version');expect(chosen.poses[0]?.centre[0]).toBe((input.fit.vertebrae.find((v)=>v.level!=='C7')?.pose.centre[0]??0)+1);
  const rods=answer.layouts[0]?.stops.find((s)=>s.stop==='rods-in');if(rods===undefined)throw new Error('stop');rods.middle=input.fit.vertebrae.map((v)=>v.pose);
  rods.rods=[{side:'left',pointsMm:[[0,0,500],[0,0,100]]}];
  const middle=rodsInDrawing(answer,batches,input.fit);expect(middle.source).toBe('middle');expect(middle.rods).toBe(rods.rods);rods.middle=null;
  rods.estimates=rods.estimates.filter((e)=>e.measure!=='main-cobb');
  batches.forEach((batch)=>{batch.runs.reverse();});
  const firstSettled=rodsInDrawing(answer,batches,input.fit);
  expect(firstSettled.source).toBe('first-version');expect(firstSettled.poses[0]?.centre[0]).toBe(chosen.poses[0]?.centre[0]);
  expect(W.fallbackFirst).toBe('The spine drawn at Rods in is one of the versions.');
  expect(rodsInDrawing(answer,[],input.fit).source).toBe('fit');
});
it('keeps at most three labels, chooses the bending anchor with placement-order ties, and omits unknown readings',()=>{
  const answer=result(),proposals=placementFixture(),fit=MADE_UP_CASE.fit;
  const labels=loadLabels(answer,proposals,fit,'standing');expect(labels.map((l)=>l.id)).toEqual(['main-cobb','kyphosis','anchor']);expect(labels[2]?.anchor).toBe(0);
  expect(loadLabels(answer,proposals,fit,'before')).toEqual([]);
  proposals.layouts[0]?.rods.forEach((r)=>{r.targetKyphosisStatus='typical';});
  expect(loadLabels(answer,proposals,fit,'standing').some((l)=>l.id==='kyphosis')).toBe(false);
  answer.mainCurve=null;proposals.curves=[];expect(loadLabels(answer,proposals,fit,'standing').map((l)=>l.id)).toEqual(['anchor']);
  expect(estimatesAt(answer,proposals,{...fit,sideFilm:null},'rods-in').some((e)=>e.measure==='apical-twist')).toBe(false);
});
it('rounds all ten estimate units outward and uses the settled denominator for run counts',()=>{
  const examples:Record<Measure,string>={'main-cobb':'1° to 2°',kyphosis:'1° to 2°','apical-twist':'1° to 2°','heaviest-anchor-force':'1 N to 2 N','heaviest-anchor-demand':'12% to 19%','heaviest-anchor-over-median':'1.2× to 1.9×','heaviest-rod-use':'12% to 19%','junction-above':'1.2× to 1.9×','junction-below':'1.2× to 1.9×','heaviest-rod-swing':'12% to 19%'};
  expect(Object.keys(ESTIMATE_NAMES)).toHaveLength(10);
  for(const measure of Object.keys(examples) as Measure[]){const percentage=['heaviest-anchor-demand','heaviest-rod-use','heaviest-rod-swing'].includes(measure);expect(rangeWords(measure,{p10:percentage?.123:1.23,p50:1.5,p90:percentage?.189:1.89})).toBe(examples[measure]);}
  expect(rangeWords('main-cobb',{p10:22,p50:22,p90:22})).toBe('about 22°');
  expect(W.denominator(5,32,3)).toBe('5 of the 29 versions that settled');expect(W.denominator(5,32,0)).toBe('5 of 32 versions');
});
it('maps only the three worker faults to retry and names a missing full fit without retry',()=>{
  const input=layoutInput(MADE_UP_CASE.fit,emptyCase()),error=(code:ConstructorParameters<typeof EngineFailure>[0]['code'])=>new EngineFailure({code,message:'',detail:null});
  for(const code of ['engine-crashed','engine-out-of-memory','engine-load-failed'] as const)expect(loadFailure(error(code),input)?.kind).toBe('failed');
  expect(loadFailure(error('pass-cancelled'),input)).toBeNull();expect(loadFailure(error('tool-refused'),{...input,fit:{...input.fit,vertebrae:[]}})).toMatchObject({kind:'refused',reason:W.missingLevels,missingLevels:true});
});

it('keeps the limits with the stage and leaves placement-comparison limits out',()=>{
  const placed=limitsForPlacement(false,true,true),loaded=limitsForPlacement(true,true,true,true);
  expect(limitsForPlacement(true,true,true).some((s)=>s.includes('Daily loads')||s.includes('A year of walking'))).toBe(false);
  expect(placed.some((s)=>s.includes('Daily loads'))).toBe(false);expect(loaded.some((s)=>s.includes('Daily loads'))).toBe(true);
  expect(placed.some((s)=>s.includes('adult’s vertebrae'))).toBe(true);expect(placed.some((s)=>s.includes('population tables'))).toBe(true);
  expect(placed.some((s)=>s.includes('Chinese'))).toBe(true);expect(placed.some((s)=>s.includes('Weight'))).toBe(false);
  expect(loaded.some((s)=>s.includes('placements are compared'))).toBe(false);expect(loaded.some((s)=>s.includes('Published shape comparisons'))).toBe(false);
  expect(limitsForPlacement(false,false,false).some((s)=>s.includes('front film only')||s.includes('With no film scale'))).toBe(false);
});

it('uses the curve tag at Rods in and each later anchor estimate’s own tag, including with no curve',()=>{
  const answer=result(),proposals=placementFixture();
  answer.layouts[0]?.stops.forEach((s)=>{s.estimates.push({measure:'heaviest-anchor-demand',band:band(.5),tag:'set-by-the-films',mostlyFrom:null,sources:[]});});
  loadState.setState({phase:'loaded',result:answer,stop:'rods-in'});
  const actions={back:vi.fn(),retry:vi.fn(),review:vi.fn(),sheet:vi.fn(),view:vi.fn(),load:vi.fn()};
  expect(placeDock(placeState.getState(),actions).small).toBe('Assumed');
  expect(placeDock(placeState.getState(),actions).helper).not.toMatchObject({words:[expect.stringContaining('Heaviest anchor')]});
  loadState.setState({stop:'standing'});expect(placeDock(placeState.getState(),actions,'patient-left',Object.keys(LOAD_STOPS) as BenchStop[]).small).toBe('Set by the films');
  answer.mainCurve=null;proposals.curves=[];placeState.setState({result:proposals});loadState.setState({phase:'loaded',result:answer,stop:'rods-in'});
  const dock=placeDock(placeState.getState(),actions);expect(dock.sentence).toBe('Rods in.');expect(dock.small).toBe('Set by the films');expect(dock.helper).toMatchObject({words:[expect.stringContaining('Heaviest anchor')]});
});
it('returns failed loads to the placement both explicitly and on leaving Place',()=>{
  const actions={back:vi.fn(),retry:vi.fn(),review:vi.fn(),sheet:vi.fn(),view:vi.fn(),load:vi.fn()};
  const failure={kind:'refused' as const,reason:W.refused,missingLevels:false};
  loadState.setState({phase:'failed',failure});
  const dock=placeDock(placeState.getState(),actions);expect(dock.sentence).toBe('This placement cannot be put under load as it stands.');expect(dock.quiet?.label).toBe('Back to the placement');dock.quiet?.press();
  expect(placeDock(placeState.getState(),actions).primary?.label).toBe(W.load);
  currentStep.getState().go('place');loadState.setState({phase:'failed',failure});currentStep.getState().go('three');expect(loadState.getState().phase).toBe('idle');
});
it('keeps empty places at placement ink, joins caps to their spans, and releases replaced rods',()=>{
  const layout=placementFixture().layouts[0];if(layout===undefined)throw new Error('layout');
  const levels=MADE_UP_CASE.fit.vertebrae.map((v)=>v.level),root=new Group(),holders=new Map(levels.map((level)=>{const g=new Group();root.add(g);return [level,g] as const;}));
  const inside=sceneLayer(root,holders,vi.fn(),vi.fn()),unregister=vi.spyOn(inside.layer,'unpointable');
  const drawn=drawPlacement({levels,addLayer:()=>inside.layer},MADE_UP_CASE.fit,layout,[]),base=drawingPoses(MADE_UP_CASE.fit,null),shift=base.map((p)=>({...p,centre:[p.centre[0],p.centre[1],p.centre[2]+40] as [number,number,number]}));
  drawn.poses(shift,false,null,undefined,true);
  const first=layout.anchors.find((a)=>a.side==='left'&&a.kind!=='none');if(first===undefined)throw new Error('head');
  const cap=inside.registrations().find((p)=>p.id==='rod:left'&&p.object instanceof Mesh)?.object;
  expect(cap?.position.z).toBeCloseTo(anchorPoint(first,MADE_UP_CASE.fit,shift)[2],8);
  drawn.poses(base,true,null);
  const rod=layout.rods[0],heads=layout.anchors.filter((a)=>a.side===rod?.side&&a.kind!=='none');if(rod===undefined||heads[0]===undefined||heads[1]===undefined)throw new Error('rod');
  const id=`span:${rod.side}:${heads[0].level}:${heads[1].level}`,segment=inside.registrations().find((p)=>p.id===id)?.object.children[0],end=inside.registrations().find((p)=>p.id===`rod:${rod.side}`&&p.object instanceof Mesh)?.object;
  if(!(segment instanceof Mesh)||!(end instanceof Mesh))throw new Error('mesh');expect(end.material).toBe(segment.material);
  const empty=layout.anchors.findIndex((a)=>a.kind==='none'),anchor=layout.anchors[empty];if(anchor===undefined)throw new Error('empty');
  drawn.loadTo([{part:{kind:'anchor',level:anchor.level,side:anchor.side},share:{p10:0,p50:0,p90:0},demand:null,versionsPastStrength:0,steady:true,sources:[]}])(1);
  const emptyMesh=inside.registrations().find((p)=>p.id===`anchor:${String(empty)}`)?.object.children[0];if(!(emptyMesh instanceof Mesh)||!(emptyMesh.material instanceof MeshBasicMaterial))throw new Error('empty mesh');expect(emptyMesh.material.opacity).toBe(.8);
  const nonempty=layout.anchors.filter((a)=>a.kind!=='none');const one=nonempty[0],two=nonempty[1];if(one===undefined||two===undefined)throw new Error('anchors');
  const own=(a:typeof one,share:number):PartResult=>({part:{kind:'anchor',level:a.level,side:a.side},share:{p10:share,p50:share,p90:share},demand:null,steady:false,versionsPastStrength:0,sources:[]});
  drawn.loadTo([own(one,.1),own(two,.2),{...own(one,.05),part:{kind:'rod-span',side:rod.side,from:heads[0].level,to:heads[1].level}}],true)(1);
  const firstMesh=inside.registrations().find((p)=>p.id===`anchor:${String(layout.anchors.indexOf(one))}`)?.object.children[0];
  if(!(firstMesh instanceof Mesh)||!(firstMesh.material instanceof MeshBasicMaterial)||!(segment.material instanceof MeshBasicMaterial))throw new Error('material');
  expect(firstMesh.material.opacity).toBe(.625);expect(segment.material.opacity).toBe(1);
  const removeListener=vi.spyOn(segment.geometry,'removeEventListener');unregister.mockClear();drawn.poses(shift,true,null);expect(unregister).toHaveBeenCalledWith(end);expect(removeListener).toHaveBeenCalledWith('dispose',expect.any(Function));
  inside.layer.onSpine.add(end);expect(inside.registrations().some((p)=>p.object===end)).toBe(false);end.removeFromParent();
  const middle=drawn.point(`rod:${rod.side}`);expect(middle?.[2]).toBeLessThan(anchorPoint(heads[0],MADE_UP_CASE.fit,shift)[2]);expect(middle?.[2]).toBeGreaterThan(anchorPoint(heads.at(-1)??heads[0],MADE_UP_CASE.fit,shift)[2]);
  drawn.remove();
});

it('withholds daily stops and their parts while preserving the full model for direct use',()=>{
  const answer=result(),rods=answer.layouts[0]?.stops.find((s)=>s.stop==='rods-in');if(rods===undefined)throw new Error('rods');
  rods.parts=structuredClone(rods.parts);
  expect(SHOWN_LOAD_STOPS).toEqual(['before','rods-in']);expect(Object.keys(LOAD_STOPS)).toHaveLength(5);
  rods.cyclesPerYear=band(1000000);rods.parts.push({part:{kind:'rod',side:'left'},share:band(.2),demand:null,steady:false,versionsPastStrength:5,sources:[]});
  rods.estimates.push({measure:'heaviest-rod-swing',band:band(.2),tag:'assumed',mostlyFrom:null,sources:[]});
  expect(shownLoadStop('bending-forward')).toBe('rods-in');
  expect(shownStopResult(answer,'year-of-walking')).toMatchObject({stop:'rods-in',cyclesPerYear:null});
  expect(shownStopResult(answer,'rods-in')?.parts.some((p)=>p.part.kind==='rod')).toBe(false);
  expect(shownStopResult(answer,'rods-in')?.estimates.some((e)=>e.measure==='heaviest-rod-swing')).toBe(false);
  expect(shownLoadStop('standing',Object.keys(LOAD_STOPS) as BenchStop[])).toBe('standing');
  const proposals=placementFixture(),part=rods.parts[1];if(part?.demand==null)throw new Error('anchor');part.demand=band(10);
  expect(loadLabels(answer,proposals,MADE_UP_CASE.fit,'rods-in','rods-in').find((l)=>l.id==='anchor')?.anchor).toBe(1);
  expect(loadLabels(answer,proposals,MADE_UP_CASE.fit,'rods-in').find((l)=>l.id==='anchor')?.anchor).toBe(0);
});

it('retains daily variation words for the full model but withholds them through the screen switch',()=>{
  for(const unknown of ['weight','steps','walking','contour-knock-down'] as const){expect(variationWords(unknown)).toBeNull();expect(variationWords(unknown,false)).toMatch(/^Varies most with:/);}
  expect(variationWords('geometry')).toBe('Varies most with: the fit.');expect(variationWords(null)).toBeNull();
});
