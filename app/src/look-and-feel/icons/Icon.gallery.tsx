import type { GalleryPage } from '@/gallery/gallery-page';
import { faultWords } from '@/words/faults';
import '../look-and-feel.gallery.css';
import { Icon } from './Icon';
import { DRAWN_ICON_NAMES, LUCIDE_ICON_NAMES, type IconName } from './icon-name';

function Named({ names, size }: { names: readonly IconName[]; size?: string }) {
  return (
    <div className={size === undefined ? 'look-gallery-set' : `look-gallery-set ${size}`}>
      {names.map((name) => (
        <span key={name} className="look-gallery-item">
          <Icon name={name} />
          <span className="look-gallery-name">{name}</span>
        </span>
      ))}
    </div>
  );
}

const SIZED: readonly IconName[] = ['check', 'close', 'undo', 'replay', 'on-device', 'case-file', 'ruler', 'magnifier'];
const SIZES: readonly { px: number; size: string }[] = [
  { px: 12, size: 'look-gallery-size-12' },
  { px: 16, size: 'look-gallery-size-16' },
  { px: 18, size: 'look-gallery-size-18' },
];

const DOCK: readonly { name: IconName; word: string; letter: string }[] = [
  { name: 'pin', word: 'Pins', letter: 'V' },
  { name: 'region-wand', word: 'Region', letter: 'R' },
  { name: 'boundary-wand', word: 'Boundary', letter: 'B' },
];

const page: GalleryPage = {
  area: 'look-and-feel',
  part: 'Icon',
  states: [
    { name: 'Drawn for this app', show: () => <Named names={DRAWN_ICON_NAMES} /> },
    { name: 'From Lucide', show: () => <Named names={LUCIDE_ICON_NAMES} /> },
    { name: 'Large, to check the drawing', show: () => <Named names={[...DRAWN_ICON_NAMES, ...LUCIDE_ICON_NAMES]} size="look-gallery-size-60" /> },
    {
      name: 'At 12, 16 and 18 px',
      show: () => (
        <div className="look-gallery-sizes">
          {SIZES.map(({ px, size }) => (
            <div key={px} className={`look-gallery-size-row ${size}`}>
              <span className="look-gallery-name">{px} px</span>
              {SIZED.map((name) => (
                <Icon key={name} name={name} />
              ))}
            </div>
          ))}
        </div>
      ),
    },
    {
      name: 'In the tool dock',
      show: () => (
        <div className="look-gallery-dock">
          {DOCK.map(({ name, word, letter }, index) => (
            <span key={name} className="look-gallery-tool" data-held={index === 0 ? 'yes' : 'no'}>
              <Icon name={name} />
              <span className="look-gallery-tool-word">{word}</span>
              <kbd>{letter}</kbd>
            </span>
          ))}
          <span className="look-gallery-dock-rule" />
          <span className="look-gallery-tool">
            <Icon name="magnifier" />
            <span className="look-gallery-tool-word">Magnify</span>
            <kbd>M</kbd>
          </span>
        </div>
      ),
    },
    {
      name: 'On the transport, 16 px',
      show: () => (
        <div className="look-gallery-transport">
          <Icon name="replay" />
          <Icon name="play" />
          <Icon name="pause" />
          <Icon name="skip" />
        </div>
      ),
    },
    {
      name: 'On the lit button, 18 px',
      show: () => (
        <>
          <span className="look-gallery-lit">
            <Icon name="boundary-wand" />
            Split T12 at the gap
          </span>
          <span className="look-gallery-lit">
            <Icon name="case-file" />
            Save the case
          </span>
        </>
      ),
    },
    {
      name: 'Large: the case tile and a file that cannot be read',
      show: () => (
        <>
          <span className="look-gallery-case-tile">
            <Icon name="case-file" />
          </span>
          <span className="look-gallery-unreadable">
            <Icon name="unreadable-file" />
            <b>{faultWords('film-unknown-format').title}</b>
            <span>Nothing else changed.</span>
          </span>
        </>
      ),
    },
  ],
};

export default page;
