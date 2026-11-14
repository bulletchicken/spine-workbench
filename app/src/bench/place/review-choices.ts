import type { LayoutProposals } from '@/shapes';
import type { DockSpec } from '@/guide/dock-spec';
import { PLACE_EDIT_WORDS as W } from '@/words/place';
import { choiceReason, endReason, type HandChange } from './hand-state';
import type { ReviewItem } from './placement-view';
export function reviewChoices(result:LayoutProposals,item:ReviewItem,press:(change:HandChange)=>void,busy:boolean):NonNullable<NonNullable<DockSpec['review']>['choices']> {
  const layout=result.layouts[0];if(layout===undefined)return[];
  const a=item.anchor===null?undefined:layout.anchors[item.anchor];
  if(a!==undefined)return ([['screw',W.keepScrew],['hook',W.useHook],['none',W.leaveEmpty]] as const).map(([kind,label])=>({label,chosen:a.kind===kind,reason:choiceReason(layout,a,kind),busy,press:()=>{if(kind!==a.kind)press({kind:'anchor',level:a.level,side:a.side,anchorKind:kind,diameterMm:null,lengthMm:null});}}));
  if(item.id==='upper'||item.id==='lower'){
    const end=item.id,drawn=layout[end],other=(end==='upper'?result.upperEnds:result.lowerEnds).find((c)=>c.level!==drawn)?.level;
    return [drawn,...other===undefined?[]:[other]].map((level)=>({label:level,chosen:level===drawn,reason:endReason(layout,end,level),busy,press:()=>{if(level!==drawn)press({kind:'end',end,level});}}));
  }
  return[];
}
