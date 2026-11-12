import { useState } from 'react';
import type { GalleryPage } from '@/gallery/gallery-page';
import './guide.gallery.css';
import { LitStandsDown } from './lit-stands-down';
import { LitButton } from './LitButton';

/** A lit button that counts its presses, by a click or by Enter. */
function Counted({ label }: { label: string }) {
  const [presses, setPresses] = useState(0);
  return (
    <div className="guide-gallery-part">
      <LitButton
        label={label}
        onPress={() => {
          setPresses((count) => count + 1);
        }}
      />
      <p className="gallery-note">Pressed {presses} times. Enter presses the lit button mounted last on this page.</p>
    </div>
  );
}

const nothing = () => undefined;

const page: GalleryPage = {
  area: 'guide',
  part: 'LitButton',
  states: [
    { name: 'The next action', show: () => <Counted label="Find the spine" /> },
    { name: 'Disabled before its time', show: () => <LitButton label="Find the spine" disabled onPress={nothing} /> },
    {
      name: 'With the icon of the tool it uses',
      show: () => (
        <div className="guide-gallery-part">
          <LitButton label="Split T12 at the gap" icon="boundary-wand" onPress={nothing} />
        </div>
      ),
    },
    {
      name: 'A long label stays on one line',
      show: () => (
        <div className="guide-gallery-part">
          <LitButton label="Go back to what was skipped earlier" icon="check" onPress={nothing} />
        </div>
      ),
    },
    {
      name: 'Stood down, while a sheet with its own lit button is open',
      show: () => (
        <div className="guide-gallery-part">
          <LitStandsDown value>
            <LitButton label="Save the case" icon="case-file" onPress={nothing} />
          </LitStandsDown>
        </div>
      ),
    },
  ],
};

export default page;
