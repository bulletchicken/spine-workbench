import { Check } from 'lucide-react';
import { useId, useState, type ReactNode } from 'react';
import type { GalleryPage } from '@/gallery/gallery-page';
import { Button } from './Button';
import { Dialog } from './Dialog';
import './Dialog.gallery.css';

const GOES_IN = [
  'Four films, under fixed names. Your file names are not kept.',
  '17 vertebrae with every corner, pedicle mark and pin',
  'Sizes, facings and which side is left, as you confirmed them',
  'Every check passes',
];

/**
 * A stand-in for the room: a strip of guide on the left and a bench with
 * four films on the right. The dialog is mounted in the bench, so its veil
 * covers the bench and leaves the guide alone. It starts closed, behind a
 * button in the guide: an open dialog holds the whole page's focus, and
 * Tab has to reach the other plates.
 */
function Room({ children }: { children: (close: () => void) => ReactNode }) {
  const [open, setOpen] = useState(false);
  const title = useId();
  return (
    <div className="blocks-gallery-room">
      <div className="blocks-gallery-guide">
        <p>The guide is outside the bench, so no veil falls on it.</p>
        <Button
          onPress={() => {
            setOpen(true);
          }}
        >
          Open the sheet
        </Button>
      </div>
      <div className="blocks-gallery-bench">
        <i />
        <i />
        <i />
        <i />
        <Dialog
          open={open}
          onClose={() => {
            setOpen(false);
          }}
          labelledBy={title}
        >
          <h3 id={title} className="blocks-gallery-sheet-title">
            Save the case.
          </h3>
          {children(() => {
            setOpen(false);
          })}
        </Dialog>
      </div>
    </div>
  );
}

const page: GalleryPage = {
  area: 'building-blocks',
  part: 'Dialog',
  states: [
    {
      name: 'A sheet over the bench',
      show: () => (
        <Room>
          {(close) => (
            <>
              <p className="blocks-gallery-sheet-sub">spine-case-2026-11-10.spinecase</p>
              <ul className="blocks-gallery-manifest">
                {GOES_IN.map((line) => (
                  <li key={line}>
                    <Check />
                    {line}
                  </li>
                ))}
              </ul>
              <p className="blocks-gallery-sheet-fine">Written to this computer and nowhere else. Extra DICOM tags are not inside it.</p>
              <div className="blocks-gallery-sheet-actions">
                <Button size="large" onPress={close}>
                  Save case file
                </Button>
                <Button size="large" onPress={close}>
                  Not yet
                </Button>
              </div>
            </>
          )}
        </Room>
      ),
    },
    {
      name: 'Focus starts on its second control',
      show: () => (
        <Room>
          {(close) => (
            <>
              <p className="blocks-gallery-sheet-fine">Focus starts on Not yet, which is marked data-autofocus.</p>
              <div className="blocks-gallery-sheet-actions">
                <Button size="large" onPress={close}>
                  Save case file
                </Button>
                <Button size="large" data-autofocus onPress={close}>
                  Not yet
                </Button>
              </div>
            </>
          )}
        </Room>
      ),
    },
    {
      name: 'Nothing inside takes focus',
      show: () => (
        <Room>
          {() => <p className="blocks-gallery-sheet-fine">Focus rests on the sheet itself. Escape closes it.</p>}
        </Room>
      ),
    },
  ],
};

export default page;
