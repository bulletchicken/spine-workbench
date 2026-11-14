import { SIDE_WORDS as W } from '@/words/side';
import type { SideNameChange } from './side-review';

export function SideReview({ onLooksRight, onRename, onLeaveOut, allowTooLow }: {
  onLooksRight: () => void;
  onRename: (change: SideNameChange) => void;
  onLeaveOut: () => void;
  allowTooLow: boolean;
}) {
  return <section className="side-controls" aria-label={W.review} onClick={(event) => { event.stopPropagation(); }}>
    <p>{W.estimated}</p>
    <button type="button" className="secondary" onClick={onLooksRight}>{W.looksRight}</button>
    <button type="button" className="quiet" onClick={() => { onRename('too-high'); }}>{W.tooHigh}</button>
    <button type="button" className="quiet" disabled={!allowTooLow} onClick={() => { onRename('too-low'); }}>{W.tooLow}</button>
    <button type="button" className="quiet" onClick={onLeaveOut}>{W.leaveOut}</button>
  </section>;
}
