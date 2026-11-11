import { FolderCheck, Plus } from 'lucide-react';
import { useEffect, useState } from 'react';
import type { GalleryPage } from '@/gallery/gallery-page';
import { Button } from './Button';
import './Button.gallery.css';
import { Kbd } from './Kbd';
import './lit-face.gallery.css';

/** A button that is busy for a moment after each press, as a save is. */
function PressToSave() {
  const [saves, setSaves] = useState(0);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    if (!busy) return;
    const done = window.setTimeout(() => {
      setBusy(false);
      setSaves((count) => count + 1);
    }, 1400);
    return () => {
      window.clearTimeout(done);
    };
  }, [busy]);
  return (
    <>
      <Button
        busy={busy}
        onPress={() => {
          setBusy(true);
        }}
      >
        <FolderCheck />
        Save case
      </Button>
      <span className="gallery-note">Saved {saves} times</span>
    </>
  );
}

const page: GalleryPage = {
  area: 'building-blocks',
  part: 'Button',
  states: [
    {
      name: 'Beside the lit button: one solid ink, one outlined, one in words',
      show: () => (
        <div className="blocks-gallery-beside-lit">
          <span className="blocks-gallery-lit">
            Save case file <Kbd keys={['Enter']} />
          </span>
          <Button size="large">Not yet</Button>
          <Button look="text">Skip for now</Button>
        </div>
      ),
    },
    {
      name: 'Standard',
      show: () => (
        <>
          <Button>
            <FolderCheck />
            Save case
          </Button>
          <Button>Not yet</Button>
          <Button>
            <Plus />
            Choose another
          </Button>
        </>
      ),
    },
    {
      name: 'Standard, disabled',
      show: () => (
        <>
          <Button disabled>
            <FolderCheck />
            Save case
          </Button>
          <Button disabled>Not yet</Button>
        </>
      ),
    },
    {
      name: 'Standard, busy',
      show: () => (
        <>
          <Button busy>
            <FolderCheck />
            Save case
          </Button>
          <Button busy>Not yet</Button>
        </>
      ),
    },
    {
      name: 'Standard, held down or open',
      show: () => (
        <>
          <Button aria-pressed>Held down</Button>
          <Button aria-expanded>Popover open</Button>
        </>
      ),
    },
    {
      name: 'Standard, large',
      show: () => (
        <>
          <Button size="large">Not yet</Button>
          <Button size="large" disabled>
            Not yet
          </Button>
          <Button size="large" busy>
            Not yet
          </Button>
        </>
      ),
    },
    {
      name: 'Text',
      show: () => (
        <div className="blocks-gallery-quiet">
          <Button look="text">Open a saved case</Button>
          <Button look="text">Try a made-up spine</Button>
        </div>
      ),
    },
    {
      name: 'Text, disabled',
      show: () => (
        <div className="blocks-gallery-quiet">
          <Button look="text" disabled>
            It really is one
          </Button>
          <Button look="text" disabled>
            Skip for now
          </Button>
        </div>
      ),
    },
    {
      name: 'Text, busy',
      show: () => (
        <div className="blocks-gallery-quiet">
          <Button look="text" busy>
            Watch again
          </Button>
        </div>
      ),
    },
    {
      // Not pulled out as the quiet rows are: these two show a face, and it stays inside the plate.
      name: 'Text, held down or open',
      show: () => (
        <>
          <Button look="text" aria-pressed>
            Held down
          </Button>
          <Button look="text" aria-expanded>
            Popover open
          </Button>
        </>
      ),
    },
    { name: 'Busy after a press', show: () => <PressToSave /> },
  ],
};

export default page;
