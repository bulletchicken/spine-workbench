import { useStore } from 'zustand';
import { LevelWords } from '@/building-blocks/LevelWords';
import { PLACE_EDIT_WORDS as W } from '@/words/place';
import { handState } from './hand-state';
import { placeState } from './place-state';
import { whyRows } from './end-rules';
export function WhyHere(){
  const hand=useStore(handState),current=useStore(placeState,(s)=>s.result),result=hand.first??current;if(hand.why===null||result===null)return null;
  return <section className="place-why"><h3>{hand.why==='upper'?W.upperEnd:W.lowerEnd}</h3><ul className="place-sheet-list">{whyRows(result,hand.why).map((row)=><li key={row}><LevelWords>{row}</LevelWords></li>)}</ul></section>;
}
