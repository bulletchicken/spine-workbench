import { beforeEach, expect, it, vi } from 'vitest';
import { TEMPLATE_LEVELS, type Template, type AnchorChoice, type ImplantLayout, type LevelRule, type RebuildInput, type RuleVote } from '@/shapes';
import { MADE_UP_CASE } from '@/spine-3d/made-up-case/made-up-case';
import { emptyCase } from '@/case-state/empty-case';
import { smallStory } from '@/engine-room/stand-in/small-story';
import { EngineFailure } from '@/engine-room/engine-client';
import { PLACE_EDIT_WORDS as W } from '@/words/place';
import { layoutInput } from './layout-input';
import { placementFixture } from './placement-fixture.scenes';
import { placeState } from './place-state';
import { loadState } from './load-state';
import { changePlacement, fittedLevels, levelOrder, openWhy, choiceReason, endReason, handState, rebuildInput, snapEnd, startFirstPlacement, undoPlacement } from './hand-state';
import { ruleCounts, ruleLanding, whyRows } from './end-rules';
import { placeDock } from './place-dock';
import { reviewItems } from './placement-view';
import { reviewChoices } from './review-choices';
let key=0;
beforeEach(()=>{placeState.setState({key:`hand-${String(key++)}`,input:layoutInput(MADE_UP_CASE.fit,emptyCase()),result:placementFixture(),phase:'ready',arriving:false,review:null,sheet:false,pressed:null,hovered:null,keyboard:false});});
const actions={back:vi.fn(),retry:vi.fn(),review:vi.fn(),sheet:vi.fn(),view:vi.fn(),undo:undoPlacement,load:vi.fn()};
function layout():ImplantLayout {const found=placeState.getState().result?.layouts[0];if(found===undefined)throw new Error('layout');return found;}
function choice(kind:AnchorChoice['kind']) {const a=layout().anchors[0];if(a===undefined)throw new Error('anchor');return {kind:'anchor' as const,level:a.level,side:a.side,anchorKind:kind,diameterMm:null,lengthMm:null};}
it('sends only the pedicles set, keeps the original input, and undo restores the preceding answer and input',async()=>{
  const original=placeState.getState().result;
  const engine={rebuildLayout:vi.fn((input:RebuildInput)=>Promise.resolve({
    ...layout(),change:{kind:'changed-by-hand' as const},upper:input.upper,lower:input.lower,
    anchors:layout().anchors.map((a)=>{const c=input.anchors.find((c)=>c.level===a.level&&c.side===a.side);return c===undefined?a:{...a,kind:c.kind,diameterMm:c.diameterMm,lengthMm:c.lengthMm};}),
  }))};
  expect(placeDock(placeState.getState(),actions).sentence).toBe('A first placement: T4 to L1.');
  const input=placeState.getState().input;await changePlacement(engine,choice('hook'));
  expect(engine.rebuildLayout.mock.calls[0]?.[0]).toMatchObject({layout:input,anchors:[{kind:'hook',diameterMm:null,lengthMm:null}]});
  expect(placeDock(placeState.getState(),actions)).toMatchObject({sentence:'Your placement: T4 to L1.',quiet:{label:'Undo'}});
  const earlier=placeState.getState().result;
  await changePlacement(engine,{kind:'end',end:'lower',level:'L2'});
  expect(engine.rebuildLayout.mock.calls[1]?.[0].anchors).toHaveLength(1);expect(engine.rebuildLayout.mock.calls[1]?.[0].layout).toBe(input);
  undoPlacement();expect(placeState.getState().result).toBe(earlier);expect(rebuildInput({kind:'end',end:'lower',level:'L2'})?.input).toEqual(engine.rebuildLayout.mock.calls[1]?.[0]);
  undoPlacement();expect(placeState.getState().result).toBe(original);expect(placeDock(placeState.getState(),actions).sentence).toBe('A first placement: T4 to L1.');expect(handState.getState().choices).toEqual([]);
  await changePlacement(engine,choice('hook'));startFirstPlacement();expect(placeState.getState().result).toBe(original);expect(handState.getState().past).toEqual([]);
});
it('forbids absent sizes and directions and the last two occupied heads, and snaps only across fitted legal ends',()=>{
  const l=layout(),a=l.anchors[0];if(a===undefined)throw new Error('anchor');a.screwHere=null;expect(choiceReason(l,a,'screw')).toBe(W.noScrew);
  a.directionStatus='missing';expect(choiceReason(l,a,'hook')).toBe(W.noDirection);
  l.anchors=l.anchors.filter((b)=>b.side!==a.side||b.level==='T4'||b.level==='T5');expect(choiceReason(l,a,'none')).toBe(W.twoAnchors);
  expect(endReason(l,'lower','T4')).toBe(W.endsApart);
  const fitted=MADE_UP_CASE.fit.vertebrae.map((v)=>v.level).filter((l)=>l!=='L2');
  expect(snapEnd(l,'lower','T4',fitted)).toBe('T6');
  expect(snapEnd(l,'lower','L2',fitted)).toBe('L1');
});
it('lands rules once, lengthens ties and suppresses level counts when a fifth of versions holds a selective test',()=>{
  const result=placementFixture();
  const rule=(name:LevelRule,end:'upper'|'lower'):RuleVote=>({rule:name,end,counts:{levels:[{level:'T4',versions:16},{level:'T5',versions:16}],unread:[]},inputs:[],flip:null,sources:[]});
  result.rules=[rule('upper-end-vertebra','upper'),rule('uev-5c','upper'),rule('end-plus-one','lower')];
  expect(ruleLanding(result.rules[0] as RuleVote)?.level).toBe('T4');expect(ruleLanding(result.rules[2] as RuleVote)?.level).toBe('T5');
  expect(ruleCounts(result,'upper')).toEqual([{level:'T4',count:2,total:2}]);expect(whyRows(result,'lower')[0]).toContain('T5 in 16 of 32 versions');
  result.selective=[{rule:'selective-thoracic',criteria:[],allHold:{kind:'share',versions:7},bothRegionsOffered:false,sources:[]}];
  result.pairs[0]?.rulesAt.push('end-plus-one');expect(ruleCounts(result,'upper')).toEqual([]);expect(whyRows(result,'upper')).toEqual(['End vertebra plus one']);
});
it('shares the chip rules with review choices and drops end items after a hand change',()=>{
  const result=placementFixture(),item=reviewItems(result).find((i)=>i.anchor!==null);if(item===undefined)throw new Error('review');
  const change=vi.fn(),choices=reviewChoices(result,item,change,false);expect(choices.map((c)=>c.label)).toEqual([W.keepScrew,W.useHook,W.leaveEmpty]);expect(choices[0]?.chosen).toBe(true);choices[1]?.press();expect(change).toHaveBeenCalledWith(expect.objectContaining({anchorKind:'hook'}));
  const l=result.layouts[0];if(l===undefined)throw new Error('layout');l.change={kind:'changed-by-hand'};expect(reviewItems(result).some((i)=>i.id==='upper'||i.id==='lower')).toBe(false);
  const names=reviewItems(result).find((i)=>i.id==='names');if(names===undefined)throw new Error('names');expect(reviewChoices(result,names,change,false)).toEqual([]);
});
it('keeps a refused answer unchanged, ignores concurrent changes, and clears a completed load after success',async()=>{
  const before=placeState.getState().result;let fail:((error:Error)=>void)|undefined;
  const engine={rebuildLayout:vi.fn(()=>new Promise<ImplantLayout>((_resolve,reject)=>{fail=reject;}))};
  const pending=changePlacement(engine,choice('hook'));await changePlacement(engine,choice('none'));expect(engine.rebuildLayout).toHaveBeenCalledOnce();
  fail?.(new EngineFailure({code:'tool-refused',message:'',detail:null}));await pending;expect(placeState.getState().result).toBe(before);expect(handState.getState().failure).toBe('refused');expect(placeDock(placeState.getState(),actions).sentence).toBe(W.failed);
  loadState.setState({phase:'loaded'});await changePlacement({rebuildLayout:()=>Promise.resolve({...layout(),change:{kind:'changed-by-hand'}})},choice('hook'));expect(loadState.getState().phase).toBe('idle');
});

function templateInput(template:Template) {
  const base=layoutInput(MADE_UP_CASE.fit,emptyCase()),marks=smallStory().pass.marks.find((m)=>m.film==='front');if(marks===undefined)throw new Error('marks');
  const v=base.fit.vertebrae.find((v)=>v.level==='T12');if(v===undefined)throw new Error('vertebra');
  const vertebrae=base.fit.vertebrae.filter((v)=>TEMPLATE_LEVELS[template].includes(v.level));
  if(template==='extra-thoracic')vertebrae.push({...v,level:'T13',pose:{...v.pose,level:'T13'}});
  return {...base,fit:{...base.fit,vertebrae},front:{marks:{...marks,naming:{...marks.naming,template}},geometry:MADE_UP_CASE.front.geometry,size:MADE_UP_CASE.front.size}};
}
it('never offers C7, including a T1 upper end, or sends an unsendable choice',async()=>{
  const l=layout();l.upper='T1';expect(fittedLevels()).not.toContain('C7');expect(levelOrder()[0]).toBe('T1');
  expect(endReason(l,'upper','C7')).toBe(W.fittedSpan);expect(snapEnd(l,'upper','C7',fittedLevels())).toBe('T1');
  const engine={rebuildLayout:vi.fn()};await changePlacement(engine,{kind:'end',end:'upper',level:'C7'});expect(engine.rebuildLayout).not.toHaveBeenCalled();expect(handState.getState().past).toEqual([]);
  const result=placeState.getState().result;if(result===null)throw new Error('result');result.upperEnds=[{level:'T1',versions:20},{level:'C7',versions:12}];
  const item={id:'upper',label:'Upper',anchor:null,levels:[],sentences:[]};expect(reviewChoices(result,item,vi.fn(),false).find((c)=>c.label==='C7')?.reason).toBe(W.fittedSpan);
});
it('uses a no-T12 naming template through T11 to L1, including review end options',()=>{
  placeState.setState({input:templateInput('missing-thoracic')});const l=layout();l.upper='T11';
  l.anchors=l.anchors.filter((a)=>a.level==='T11'||a.level==='L1');
  expect(levelOrder()).not.toContain('T12');expect(endReason(l,'lower','L1')).toBeNull();expect(snapEnd(l,'lower','L1',fittedLevels())).toBe('L1');
  expect(rebuildInput({kind:'end',end:'lower',level:'L1'})?.input).toMatchObject({upper:'T11',lower:'L1'});
  const result=placeState.getState().result;if(result===null)throw new Error('result');result.upperEnds=[{level:'T11',versions:20},{level:'T12',versions:12}];
  const item={id:'upper',label:'Upper',anchor:null,levels:[],sentences:[]};expect(reviewChoices(result,item,vi.fn(),false).find((c)=>c.label==='T12')?.reason).toBe(W.fittedSpan);
});
it('sends and records T13 choices under the film template and rejects them under a standard count',async()=>{
  const input=templateInput('extra-thoracic'),l=layout(),model=l.anchors[0];if(model===undefined)throw new Error('anchor');
  l.anchors.push({...model,level:'T13'});placeState.setState({input});
  const change={kind:'anchor' as const,level:'T13' as const,side:model.side,anchorKind:'hook' as const,diameterMm:null,lengthMm:null};
  const engine={rebuildLayout:vi.fn((sent:RebuildInput)=>Promise.resolve({...l,upper:sent.upper,lower:sent.lower,change:{kind:'changed-by-hand' as const}}))};
  await changePlacement(engine,change);expect(engine.rebuildLayout.mock.calls[0]?.[0].anchors).toEqual([expect.objectContaining({level:'T13',kind:'hook'})]);expect(handState.getState().choices).toHaveLength(1);
  const result=placeState.getState().result;placeState.setState({key:'standard-reject',input:templateInput('standard'),result});
  const refused={rebuildLayout:vi.fn()};await changePlacement(refused,change);expect(refused.rebuildLayout).not.toHaveBeenCalled();expect(handState.getState().past).toEqual([]);
});
it('forgets a choice removed by a shorter span; expansion follows the engine again and undo restores it',async()=>{
  const engine={rebuildLayout:vi.fn((input:RebuildInput)=>Promise.resolve({...layout(),upper:input.upper,lower:input.lower,change:{kind:'changed-by-hand' as const}}))};
  await changePlacement(engine,choice('hook'));expect(handState.getState().choices).toHaveLength(1);
  await changePlacement(engine,{kind:'end',end:'upper',level:'T5'});expect(handState.getState().choices).toEqual([]);
  undoPlacement();expect(handState.getState().choices).toHaveLength(1);
  await changePlacement(engine,{kind:'end',end:'upper',level:'T5'});await changePlacement(engine,{kind:'end',end:'upper',level:'T4'});
  expect(engine.rebuildLayout.mock.calls.at(-1)?.[0].anchors).toEqual([]);expect(handState.getState().choices).toEqual([]);
});
it('clears Why here on every sheet close and does not resurrect selections changed during a rebuild',async()=>{
  openWhy('lower');placeState.setState({sheet:false});expect(handState.getState().why).toBeNull();
  for(const selection of [null,'anchor:1']){
    placeState.setState({pressed:'anchor:0',hovered:'anchor:3'});let finish:((value:ImplantLayout)=>void)|undefined;
    const before=layout(),engine={rebuildLayout:()=>new Promise<ImplantLayout>((resolve)=>{finish=resolve;})};
    const pending=changePlacement(engine,choice('hook'));placeState.setState({pressed:selection,hovered:selection===null?null:'anchor:2'});
    const answer={...before,anchors:[...before.anchors].reverse(),change:{kind:'changed-by-hand' as const}};finish?.(answer);await pending;
    const expected=selection===null?null:`anchor:${String(answer.anchors.findIndex((a)=>a===before.anchors[1]))}`;
    expect(placeState.getState().pressed).toBe(expected);expect(placeState.getState().hovered).toBe(selection===null?null:`anchor:${String(answer.anchors.findIndex((a)=>a===before.anchors[2]))}`);
  }
});
