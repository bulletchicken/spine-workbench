import {expect,it} from 'vitest';
import {putOnThePage} from '@/building-blocks/test-rig/on-the-page';
import {PlaceSources} from './PlaceSources';
import {sourceRecords,type PlaceEvidence} from './placement-sources';
it('lists one source once across numbers, with only its recorded population, sample and reading',()=>{
 const entry:PlaceEvidence={id:'place-layout-one',feature:'Width',publishedNumber:'3 mm',population:'children aged 9 to 11',sampleSize:'30 children',sources:[{key:'table',reading:'abstract',citation:'A table paper',pmid:null,doi:null}],checked:false,status:'experimental'};
 const page=putOnThePage(<PlaceSources records={sourceRecords([{key:'table',from:'layout'}],[entry,{...entry,id:'place-layout-two',feature:'Length'}])}/>);
 expect(page.host.querySelectorAll('.place-sources > ul > li')).toHaveLength(1);expect(page.host.textContent).toContain('Abstract on record');expect(page.host.textContent).toContain('Sample: 30 children');expect(page.host.textContent).toContain('Not yet checked against the paper.');
 const {sampleSize:omitted,...withoutSample}=entry;expect(omitted).toBe('30 children');
 page.show(<PlaceSources records={sourceRecords([{key:'table',from:'layout'}],[withoutSample])}/>);
 expect(page.host.textContent).not.toContain('Sample:');
 page.show(<PlaceSources records={sourceRecords([{key:'table',from:'layout'}],[{...entry,sources:[{key:'another',citation:'First paper',pmid:null,doi:null},...entry.sources]}])}/>);expect(page.host.textContent).not.toContain('Sample: 30 children');expect(page.host.textContent).not.toContain('Full text');page.show(<PlaceSources records={sourceRecords([{key:'project-assumptions',from:'bench'}])}/>);expect(page.host.textContent).toContain("The project's own assumption");expect(page.host.querySelector('button, a')).toBeNull();page.takeDown();
});

it('keeps citation levels in mono and kinds after papers, without raw population tags',()=>{
 const entry:PlaceEvidence={id:'place-layout-one',feature:'Width',publishedNumber:'3',population:'paper: Adults. Sample: 30 [peer-reviewed; full-text].',sampleSize:'30',sources:[{key:'paper',reading:'full-text',citation:'Study of T4 and T7',pmid:null,doi:null}],checked:false,status:'experimental'};
 const page=putOnThePage(<PlaceSources records={sourceRecords([{key:'project-assumptions',from:'layout'},{key:'paper',from:'layout'}],[entry])}/>);
 expect(page.host.querySelector('.place-sources > ul > li')?.textContent).toContain('Study of T4 and T7');expect(page.host.querySelectorAll('.level-name')).toHaveLength(2);expect(page.host.textContent).not.toContain('paper:');expect(page.host.textContent).not.toContain('[peer-reviewed');expect(page.host.textContent).toContain('Sample: 30');page.takeDown();
});
