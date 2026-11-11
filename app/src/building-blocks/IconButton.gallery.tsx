import { Pause, Redo2, RotateCcw, SkipForward, Undo2 } from 'lucide-react';
import { useState } from 'react';
import type { GalleryPage } from '@/gallery/gallery-page';
import './glass-strip.gallery.css';
import { IconButton } from './IconButton';

/** Undo and redo over a short history, so each tooltip names the action a press would do. */
function UndoAndRedo() {
  const done = ['Split T12', 'Add to T3', 'Confirm T1'];
  const [at, setAt] = useState(2);
  return (
    <>
      <IconButton
        label="Undo"
        tip={at === 0 ? 'Nothing to undo' : `Undo: ${done[at - 1] ?? ''}`}
        tipAlign="start"
        disabled={at === 0}
        onPress={() => {
          setAt(at - 1);
        }}
      >
        <Undo2 />
      </IconButton>
      <IconButton
        label="Redo"
        tip={at === done.length ? 'Nothing to redo' : `Redo: ${done[at] ?? ''}`}
        tipAlign="start"
        disabled={at === done.length}
        onPress={() => {
          setAt(at + 1);
        }}
      >
        <Redo2 />
      </IconButton>
    </>
  );
}

const page: GalleryPage = {
  area: 'building-blocks',
  part: 'IconButton',
  states: [
    {
      name: 'Square, its tooltip naming the action',
      show: () => (
        <>
          <IconButton label="Undo" tip="Undo: Split T12" tipAlign="start">
            <Undo2 />
          </IconButton>
          <IconButton label="Redo" tip="Redo: Add to T3" tipAlign="start">
            <Redo2 />
          </IconButton>
        </>
      ),
    },
    {
      name: 'Disabled, its tooltip saying why',
      show: () => (
        <>
          <IconButton label="Undo" tip="Nothing to undo" tipAlign="start" disabled>
            <Undo2 />
          </IconButton>
          <IconButton label="Redo" tip="Nothing to redo" tipAlign="start" disabled>
            <Redo2 />
          </IconButton>
        </>
      ),
    },
    {
      name: 'With its name for a tooltip',
      show: () => (
        <IconButton label="Watch again from the start" tipAlign="start">
          <RotateCcw />
        </IconButton>
      ),
    },
    {
      name: 'Held down or open',
      show: () => (
        <>
          <IconButton label="Held down" tipAlign="start" aria-pressed>
            <Pause />
          </IconButton>
          <IconButton label="Popover open" tipAlign="start" aria-expanded>
            <Undo2 />
          </IconButton>
        </>
      ),
    },
    {
      name: 'Round, on glass',
      show: () => (
        <div className="blocks-gallery-glass">
          <IconButton shape="round" label="Watch again from the start" tipSide="above" tipAlign="start">
            <RotateCcw />
          </IconButton>
          <IconButton shape="round" label="Pause" tipSide="above">
            <Pause />
          </IconButton>
          <IconButton shape="round" label="Playback speed" tip="Speed: 1×. Press for 2×" tipSide="above">
            1×
          </IconButton>
          <IconButton shape="round" label="Skip to the result" tipSide="above" disabled>
            <SkipForward />
          </IconButton>
        </div>
      ),
    },
    {
      name: 'Round, held down or open',
      show: () => (
        <div className="blocks-gallery-glass">
          <IconButton shape="round" label="Held down" tipSide="above" tipAlign="start" aria-pressed>
            <Pause />
          </IconButton>
          <IconButton shape="round" label="Popover open" tipSide="above" aria-expanded>
            1×
          </IconButton>
        </div>
      ),
    },
    { name: 'Undo and redo, walking a history', show: () => <UndoAndRedo /> },
  ],
};

export default page;
