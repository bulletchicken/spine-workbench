import { Check, Info } from 'lucide-react';
import type { GalleryPage } from '@/gallery/gallery-page';
import { BLOCK_WORDS } from '@/words/blocks';
import { Button } from '../Button';
import { say, type Said } from './say';
import { Toast } from './Toast';
import './Toast.gallery.css';
import { Toasts } from './Toasts';

const nothing = () => undefined;
const UNDO = { label: BLOCK_WORDS.undo, keys: ['Mod', 'z'], press: nothing };
// The two marks, as the bench will hand them in. Drawn a little heavier, so a 16 px tick weighs what the design's does.
const MARKS = { done: <Check strokeWidth={2.2} />, note: <Info strokeWidth={2.2} /> };

/** A toast held on show, so its look can be studied. */
function Held({ line, tone = 'done', action = null }: Pick<Said, 'line'> & Partial<Pick<Said, 'tone' | 'action'>>) {
  return (
    <div className="blocks-gallery-held-toast">
      <Toast said={{ id: 0, line, tone, action }} shown mark={MARKS[tone]} onAction={nothing} onStayed={nothing} />
    </div>
  );
}

/** The foot of a bench, with buttons that say things. */
function Bench() {
  return (
    <div className="blocks-gallery-toast-bench">
      <div className="blocks-gallery-toast-asks">
        <Button
          onPress={() => {
            say('Split T12 in two. Levels renamed to match.', {
              action: {
                ...UNDO,
                press: () => {
                  say('Undid: split T12.');
                },
              },
            });
          }}
        >
          Split T12
        </Button>
        <Button
          onPress={() => {
            say('Left and right confirmed. It matches the lead marker.');
          }}
        >
          Answer left and right
        </Button>
        <Button
          onPress={() => {
            say('No disc gap found inside T8.', { tone: 'note' });
          }}
        >
          Click where there is no gap
        </Button>
      </div>
      <Toasts marks={MARKS} />
    </div>
  );
}

const page: GalleryPage = {
  area: 'building-blocks',
  part: 'Toast',
  states: [
    { name: 'Something done, with Undo', show: () => <Held line="L5 set. The one below it is the sacrum. Levels renamed." action={UNDO} /> },
    { name: 'Something done', show: () => <Held line="Size set from your two clicks: 0.151 mm per pixel." /> },
    { name: 'A note: nothing changed', show: () => <Held line="No disc gap found inside T8." tone="note" /> },
    {
      name: 'Said at the foot of the bench',
      show: () => <Bench />,
    },
  ],
};

export default page;
