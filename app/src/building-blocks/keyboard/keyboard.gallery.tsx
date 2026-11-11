import { useEffect, useState } from 'react';
import type { GalleryPage } from '@/gallery/gallery-page';
import { Button } from '../Button';
import { Kbd } from '../Kbd';
import { KEY_RANKS, type KeyRank } from './key-ranks';
import { stopKeysAt } from './keyboard';
import './keyboard.gallery.css';
import { useKey } from './use-key';

// One stand-in taker for each rank, answering the keys that rank answers in the app.
const TAKERS: Record<KeyRank, { part: string; keys: readonly (readonly string[])[] }> = {
  always: { part: 'Undo, redo and save', keys: [['Mod', 'z'], ['Mod', 'Shift', 'z'], ['Mod', 's']] },
  sheet: { part: 'An open dialog, which keeps every key from the ranks below', keys: [['Escape'], ['Enter']] },
  popover: { part: 'An open popover', keys: [['Escape']] },
  gesture: { part: 'A drag under way', keys: [['Escape']] },
  choice: { part: 'A choice on screen', keys: [['a'], ['b'], ['ArrowLeft'], ['ArrowRight']] },
  lit: { part: 'The lit button', keys: [['Enter']] },
  review: { part: 'The review', keys: [['Escape']] },
  pass: { part: 'The pass', keys: [['Space']] },
  tools: { part: 'The tool dock', keys: [['v'], ['r'], ['b'], ['m']] },
  cameras: { part: 'The cameras', keys: [['0'], ['+'], ['-']] },
};

const STARTS_ON: readonly KeyRank[] = ['always', 'lit', 'review', 'pass', 'tools', 'cameras'];

/** A taker that is registered only while its row is switched on. */
function Taker({ rank, onTake }: { rank: KeyRank; onTake: (key: string) => void }) {
  const pressed = TAKERS[rank].keys.map((keys) => keys.at(-1) ?? '');
  useKey(rank, pressed, (event) => {
    // Undo, redo and save are theirs only with Cmd or Ctrl held.
    if (rank === 'always' && !(event.metaKey || event.ctrlKey)) return false;
    onTake(event.key === ' ' ? 'Space' : event.key);
    return true;
  });
  // As the Dialog block does while it is open.
  useEffect(() => (rank === 'sheet' ? stopKeysAt('sheet') : undefined), [rank]);
  return null;
}

/** The ten ranks in order. Switch parts on and off, press a key, and see who took it. */
function WhoTakesAKey() {
  const [on, setOn] = useState<ReadonlySet<KeyRank>>(new Set(STARTS_ON));
  const [taken, setTaken] = useState<{ rank: KeyRank; key: string; count: number } | null>(null);
  return (
    <div className="blocks-gallery-ranks">
      <p className="gallery-note" role="status">
        {taken === null ? 'Press a key anywhere on this page.' : `${taken.key} was taken by “${taken.rank}” (${String(taken.count)} taken so far).`}
      </p>
      <ol>
        {KEY_RANKS.map((rank) => (
          <li key={rank} data-on={on.has(rank) || undefined} data-took={taken?.rank === rank || undefined}>
            {on.has(rank) && (
              <Taker
                rank={rank}
                onTake={(key) => {
                  setTaken((before) => ({ rank, key, count: (before?.count ?? 0) + 1 }));
                }}
              />
            )}
            <Button
              look="text"
              aria-pressed={on.has(rank)}
              onPress={() => {
                const next = new Set(on);
                if (!next.delete(rank)) next.add(rank);
                setOn(next);
              }}
            >
              {rank}
            </Button>
            <span>{TAKERS[rank].part}</span>
            <span className="blocks-gallery-rank-keys">
              {TAKERS[rank].keys.map((keys) => (
                <Kbd key={keys.join(' ')} keys={keys} />
              ))}
            </span>
          </li>
        ))}
      </ol>
      <label className="gallery-note">
        While typing here, only “always” is asked: <input type="text" aria-label="A field to type in" />
      </label>
    </div>
  );
}

const page: GalleryPage = {
  area: 'building-blocks',
  part: 'Key ranks',
  states: [{ name: 'Who takes a key', show: () => <WhoTakesAKey /> }],
};

export default page;
