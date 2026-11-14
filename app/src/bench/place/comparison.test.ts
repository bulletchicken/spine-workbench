import {placeDock} from './place-dock';
import {EngineFailure} from '@/engine-room/engine-client';
import {afterEach,beforeEach,expect,it,vi} from 'vitest';
import type {BenchBatch,BenchCompareInput,BenchInput,BenchResult,PairedCount,LayoutInput,LayoutProposals} from '@/shapes';
import {emptyCase} from '@/case-state/empty-case';
import {MADE_UP_CASE} from '@/spine-3d/made-up-case/made-up-case';
import {layoutInput} from './layout-input';
import {placementFixture} from './placement-fixture.scenes';
import {comparisonLayouts,comparisonName,pairedWords,comparisonDifferences,hardwareKey,comparisonMeasures,comparisonPageSize} from './comparison-view';
import {comparisonState,comparePlacements,stopComparison,closeComparison} from './comparison-state';
import {placeState,changeAge} from './place-state';
import {handState} from './hand-state';
import {loadState} from './load-state';
import {ageOrigin,pedicleOrigin,sourcesFor,stageSourceUses,sourceRecords,populationWords,endSourceKeys,type PlaceEvidence} from './placement-sources';
let key=0;
function setup(){
 const proposals=placementFixture(),first=proposals.layouts[0];if(first===undefined)throw new Error('first');
 const other=structuredClone(first);other.id='fewer';other.change={kind:'fewer-screws'};if(other.anchors[0]!==undefined)other.anchors[0].kind='none';proposals.layouts.push(other);
 const input={...layoutInput(MADE_UP_CASE.fit,emptyCase()),versions:8},baseline:BenchResult={versions:8,firstId:first.id,mainCurve:null,scales:[],layouts:[{layoutId:first.id,versionsUnsettled:0,counts:[],stops:[]}]};
 placeState.setState({key:String(key++),result:proposals,input,ageYears:null,phase:'ready',arriving:false});
 loadState.setState({phase:'loaded',result:baseline,batches:[{versionFrom:0,versionCount:8,runs:[],middle:[],mainCurve:null}]});return{proposals,first,other,input,baseline};
}
beforeEach(()=>{placeState.setState({key:String(key++)});});
afterEach(()=>{vi.useRealTimers();});
it('keeps the stage placement first, omits identical hardware, and names changes from the first',()=>{
 const {proposals,first,other}=setup(),copy={...first,id:'copy',change:{kind:'stiffer-rods'} as const};proposals.layouts.push(copy);
 expect(comparisonDifferences(first,copy)).toEqual([]);
 expect(comparisonLayouts(first,proposals).map((l)=>l.id)).toEqual([first.id,other.id]);
 expect(comparisonName(first,first)).toBe('The first placement');expect(comparisonName(other,first)).toBe('Fewer screws');
 const yours={...structuredClone(other),id:'changed-by-hand',change:{kind:'changed-by-hand'} as const};if(yours.anchors[1]!==undefined)yours.anchors[1].kind='hook';
 expect(comparisonLayouts(yours,proposals)[0]).toBe(yours);expect(comparisonName(yours,first)).toBe('Yours');
 expect(comparisonName({...first,change:{kind:'starts-at',level:'T3'}},first)).toBe('Starts one level higher');
 expect(comparisonName({...first,change:{kind:'ends-at',level:'L2'}},first)).toBe('Ends one level lower');
});
it('reads dot order, ties and the engine threshold without recomputing them',()=>{
 const count:PairedCount={measure:'main-cobb',stop:'rods-in',lower:3,higher:0,ties:1,clearAt:3,order:[7,2,4,0],outcomes:['tie','lower','lower','lower'],sortedBy:null};
 expect(pairedWords(count)).toEqual({verdict:'Lower in 3 of 4 versions',clear:'clear at 3',dots:count.outcomes});
 expect(pairedWords({...count,lower:1,higher:3})).toMatchObject({verdict:'Higher in 3 of 4 versions'});
 expect(pairedWords({...count,lower:2})).toMatchObject({verdict:'No clear difference'});expect(pairedWords(undefined)).toEqual({verdict:'Not measured for this placement.',clear:'',dots:[]});
});
it('loads single placements in covering slices, compares baseline first, and leaves Loaded untouched',async()=>{
 const {input,baseline,first,other}=setup();
 const engine={runLoadBench:vi.fn<(i:BenchInput)=>Promise<BenchBatch>>().mockImplementation((i)=>Promise.resolve({versionFrom:i.versionFrom,versionCount:i.versionCount,runs:[],middle:[],mainCurve:null})),compareLayouts:vi.fn<(i:BenchCompareInput)=>Promise<BenchResult>>().mockImplementation((i)=>Promise.resolve({...baseline,layouts:i.layouts.map((l)=>({layoutId:l.id,versionsUnsettled:0,counts:[],stops:[]}))}))};
 await comparePlacements(engine);expect(engine.runLoadBench.mock.calls.map(([i])=>[i.layouts.map((l)=>l.id),i.versionFrom,i.versionCount])).toEqual([[[other.id],0,4],[[other.id],4,4]]);
 expect(engine.runLoadBench.mock.calls[0]?.[0]).toMatchObject({fit:input.fit,child:input.child,seed:input.seed,versions:8});
 expect(engine.compareLayouts.mock.calls[0]?.[0].layouts).toEqual([first,other]);expect(comparisonState.getState().finished).toHaveLength(2);expect(loadState.getState().result).toBe(baseline);
 await comparePlacements(engine);expect(engine.runLoadBench).toHaveBeenCalledTimes(2);
});
it('Stop keeps finished columns and rejects an unfinished slice and every following placement',async()=>{
 const {proposals,other,baseline}=setup();const third=structuredClone(other);third.id='third';const rod=third.rods[0];if(rod===undefined)throw new Error('rod');rod.diameterMm+=1;proposals.layouts.push(third);
 let release:((batch:BenchBatch)=>void)|undefined;
 const engine={runLoadBench:vi.fn<(i:BenchInput)=>Promise<BenchBatch>>().mockImplementation(async(i)=>i.layouts[0]?.id==='third'?new Promise((resolve)=>{release=resolve;}):{versionFrom:i.versionFrom,versionCount:i.versionCount,runs:[],middle:[],mainCurve:null}),compareLayouts:vi.fn().mockResolvedValue(baseline)};
 const pending=comparePlacements(engine);await vi.waitFor(()=>{expect(release).toBeDefined();});comparisonState.setState({page:4});stopComparison();release?.({versionFrom:0,versionCount:4,runs:[],middle:[],mainCurve:null});await pending;
 expect(comparisonState.getState().finished.map((l)=>l.id)).toEqual([proposals.layouts[0]?.id,other.id]);expect(comparisonState.getState().batches).toHaveLength(3);expect(comparisonState.getState().page).toBe(0);expect(engine.compareLayouts).toHaveBeenCalledOnce();
});
it('age re-proposes with no weight or activity, dropping hand choices, load and comparison results',async()=>{
 const {proposals}=setup();handState.setState({choices:[{level:'T4',side:'left',kind:'hook',diameterMm:null,lengthMm:null}],past:[{result:proposals,choices:[]}],first:proposals});
 const engine={proposeLayouts:vi.fn<(input:LayoutInput)=>Promise<LayoutProposals>>().mockResolvedValue(proposals)},accepted={key:'age',fit:MADE_UP_CASE.fit,bones:[],input:{} as never};
 await changeAge(engine,accepted,emptyCase(),9);expect(engine.proposeLayouts.mock.calls[0]?.[0].child).toEqual({ageYears:9,weightKg:null,activity:null});expect(handState.getState().past).toEqual([]);expect(handState.getState().choices).toEqual([]);expect(loadState.getState().result).toBeNull();expect(comparisonState.getState().result).toBeNull();expect(placeState.getState().ageChanged).toBe(true);
});
it('matches only explicit ledger keys and uses populations as given, with age fallbacks and unique sources',()=>{
 const {proposals}=setup(),a=proposals.layouts[0]?.anchors[0];if(a===undefined)throw new Error('anchor');
 const entry:PlaceEvidence={id:'place-layout-pedicle-widths-table-outer-width',feature:'Width',publishedNumber:'3 mm',population:'girls of about 15 with scoliosis and matched controls',sampleSize:'30 girls',sources:[{citation:'A paper',pmid:null,doi:null,key:'width-table',reading:'full-text'}],checked:false,status:'experimental'};
 a.pedicle.sources=['width-table','width-table'];expect(pedicleOrigin(a,null,[{...entry,id:'place-layout-chords-wrong-table',population:'a different table'},entry])).toBe('from a table of girls of about 15 with scoliosis and matched controls');expect(sourcesFor(['width-table','width-table'],'layout',[entry])).toEqual([entry]);
 const bench={...entry,id:'place-bench-width',sampleSize:'Other sample'};expect(sourcesFor(['width-table'],'layout',[bench,entry])).toEqual([entry]);expect(sourcesFor(['width-table'],'bench',[bench,entry])).toEqual([bench]);expect(sourceRecords([{key:'project-assumptions',from:'bench'}])[0]?.kind).toBe('project-assumption');
 expect(pedicleOrigin(a,null,[])).toBe('from tables for teenagers: no age was given');expect(ageOrigin('T7',9)).toBe('from published tables');expect(ageOrigin('T7',5)).toBe('from published tables');expect(ageOrigin('L1',9)).toBe('from published tables');expect(ageOrigin('T7',12)).toBe('from tables for teenagers');
 expect(stageSourceUses(proposals,null).filter((use)=>use.key==='width-table')).toHaveLength(1);a.pedicle.outerWidthMm=null;expect(pedicleOrigin(a,null,[entry])).toBeNull();
});

it('discards running comparisons on Done or leaving but retains an explicit Stop',async()=>{
 vi.useFakeTimers();const {baseline}=setup();let release:((batch:BenchBatch)=>void)|undefined;
 const engine={runLoadBench:vi.fn(()=>new Promise<BenchBatch>((resolve)=>{release=resolve;})),compareLayouts:vi.fn().mockResolvedValue(baseline)};
 const pending=comparePlacements(engine);closeComparison();vi.advanceTimersByTime(640);expect(comparisonState.getState().phase).toBe('idle');release?.({versionFrom:0,versionCount:4,runs:[],middle:[],mainCurve:null});await pending;
 const again=comparePlacements(engine);expect(engine.runLoadBench).toHaveBeenCalledTimes(2);stopComparison();closeComparison(true);expect(comparisonState.getState().phase).toBe('done');release?.({versionFrom:0,versionCount:4,runs:[],middle:[],mainCurve:null});await again;
});
it('can retry engine faults and drops non-retryable comparisons',async()=>{
 const {baseline}=setup(),engine={runLoadBench:vi.fn().mockRejectedValue(new EngineFailure({code:'engine-crashed',message:'',detail:null})),compareLayouts:vi.fn().mockResolvedValue(baseline)};
 await comparePlacements(engine);expect(comparisonState.getState()).toMatchObject({phase:'failed',retryable:true});await comparePlacements(engine);expect(engine.runLoadBench).toHaveBeenCalledTimes(2);
 engine.runLoadBench.mockRejectedValue(new EngineFailure({code:'tool-refused',message:'',detail:null}));await comparePlacements(engine);expect(comparisonState.getState()).toMatchObject({phase:'idle',open:false});
});
it('only offers anchor comparisons without a main curve and keys rods on the bench input',()=>{
 const {first,baseline}=setup();expect(comparisonMeasures(baseline)).toEqual(['heaviest-anchor-demand']);comparisonState.setState({open:true,phase:'done'});expect(placeDock(placeState.getState(),{back:vi.fn(),retry:vi.fn(),review:vi.fn(),sheet:vi.fn(),view:vi.fn()}).control?.options.map((option)=>option.id)).toEqual(['heaviest-anchor-demand']);expect(comparisonMeasures({...baseline,mainCurve:{upper:'T4',apex:'T7',lower:'L1'}})).toEqual(['main-cobb','heaviest-anchor-demand']);
 const changed=structuredClone(first),rod=changed.rods[0];if(rod===undefined)throw new Error('rod');rod.standingPointsMm=[];expect(hardwareKey(changed)).toBe(hardwareKey(first));rod.overbendDeg+=1;expect(hardwareKey(changed)).not.toBe(hardwareKey(first));
 expect(comparisonPageSize(1280)).toBe(3);expect(comparisonPageSize(1440)).toBe(3);expect(comparisonPageSize(640)).toBe(1);
});
it('cleans each paper population separately and gives an end only its own rules',()=>{
 const {proposals}=setup(),entry:PlaceEvidence={id:'place-bench-test',feature:'test',publishedNumber:'1',population:'one: Adults at T4. Sample: 6 [peer-reviewed; full-text]. two: Children at T7. Sample: 3 [peer-reviewed; abstract].',sources:[{key:'one',citation:'First',pmid:null,doi:null},{key:'two',citation:'Second',pmid:null,doi:null}],checked:false,status:'experimental'};
 expect(populationWords(entry,'one')).toBe('Adults at T4.');expect(populationWords(entry,'two')).toBe('Children at T7.');expect(populationWords(entry,'missing')).toBe('');
 const overlapping={...entry,population:'winter-via-keenan-2015: Adults. Sample: 8 [secondary]. keenan-2015: Girls. Sample: 20 [full-text].',sources:[{key:'winter-via-keenan-2015',citation:'Winter',pmid:null,doi:null},{key:'keenan-2015',citation:'Keenan',pmid:null,doi:null}]};expect(populationWords(overlapping,'winter-via-keenan-2015')).toBe('Adults.');expect(populationWords(overlapping,'keenan-2015')).toBe('Girls.');
 proposals.rules.forEach((rule)=>{rule.sources=[rule.end==='upper'?'upper-only':'lower-only'];});expect(endSourceKeys(proposals,'upper')).toEqual(['upper-only']);expect(endSourceKeys(proposals,'lower')).toEqual(['lower-only']);
});

it('does not start a comparison for a baseline with too many unsettled versions',async()=>{
 const {baseline}=setup(),layout=baseline.layouts[0];if(layout===undefined)throw new Error('layout');layout.versionsUnsettled=9;
 const engine={runLoadBench:vi.fn(),compareLayouts:vi.fn()};await comparePlacements(engine);expect(engine.runLoadBench).not.toHaveBeenCalled();expect(comparisonState.getState().open).toBe(false);
});
