import { type LayoutProposals, type Level, type RuleVote } from '@/shapes';
import { LEVEL_RULE_NAMES as N, PLACE_EDIT_WORDS as W } from '@/words/place';
import { levelOrder, type End } from './hand-state';
export function selectiveRules(result:LayoutProposals):boolean {return result.selective.some((s)=>s.allHold.kind==='share'&&s.allHold.versions>=result.versions/5);}
export function ruleLanding(rule:RuleVote):RuleVote['counts']['levels'][number]|null {
  const order=levelOrder();
  return rule.counts.levels.filter((l)=>l.versions>0).reduce<RuleVote['counts']['levels'][number]|null>((best,l)=>best===null||l.versions>best.versions||l.versions===best.versions&&(rule.end==='upper'?order.indexOf(l.level)<order.indexOf(best.level):order.indexOf(l.level)>order.indexOf(best.level))?l:best,null);
}
export function ruleCounts(result:LayoutProposals,end:End):{level:Level;count:number;total:number}[] {
  if(selectiveRules(result))return[];
  const readable=result.rules.filter((r)=>r.end===end).map(ruleLanding).filter((r)=>r!==null);
  return [...new Set(readable.map((r)=>r.level))].map((level)=>({level,count:readable.filter((r)=>r.level===level).length,total:readable.length}));
}
export function whyRows(result:LayoutProposals,end:End):string[] {
  if(selectiveRules(result)){const pair=result.pairs[0];return pair===undefined?[]:[...new Set([...pair.rulesAt,...pair.rulesNear,...pair.rulesAgainst])].map((rule)=>N[rule]);}
  return result.rules.filter((r)=>r.end===end).map((r)=>{const landing=ruleLanding(r);return landing===null?W.ruleUnread(N[r.rule]):W.ruleAt(N[r.rule],landing.level,landing.versions,result.versions);});
}
