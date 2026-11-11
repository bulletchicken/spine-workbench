import { Pause, RotateCcw, SkipForward } from 'lucide-react';
import { useState } from 'react';
import type { GalleryPage } from '@/gallery/gallery-page';
import './playback-strip.gallery.css';
import { IconButton } from './IconButton';
import { Slider } from './Slider';
import type { SliderSegment } from './slider-geometry';
import './Slider.gallery.css';

// The six beats of the pass, in milliseconds, as the design times them.
const BEATS: readonly SliderSegment[] = [
  { from: 400, to: 2400, label: 'Evidence' },
  { from: 2400, to: 3700, label: 'Centre line' },
  { from: 3700, to: 5400, label: 'Unroll' },
  { from: 5400, to: 9000, label: 'Search' },
  { from: 9000, to: 10400, label: 'Lock in' },
  { from: 10400, to: 11600, label: 'Confidence' },
];

function beatAt(time: number): string {
  if (time <= 400) return 'Starting';
  if (time >= 11600) return 'Finished';
  return BEATS.find((beat) => time < beat.to)?.label ?? 'Finished';
}

/** The slider as playback holds it: in its ruled strip, between round buttons. */
function Playback({ startAt, disabled = false }: { startAt: number; disabled?: boolean }) {
  const [time, setTime] = useState(startAt);
  return (
    <div className="blocks-gallery-strip blocks-gallery-playback">
      <IconButton
        shape="round"
        label="Watch again from the start"
        tipSide="above"
        tipAlign="start"
        onPress={() => {
          setTime(400);
        }}
      >
        <RotateCcw />
      </IconButton>
      <IconButton shape="round" label="Pause" tipSide="above">
        <Pause />
      </IconButton>
      <Slider label="Position in the automatic pass" segments={BEATS} value={time} step={500} valueText={beatAt} disabled={disabled} onChange={setTime} />
      <IconButton
        shape="round"
        label="Skip to the result"
        tipSide="above"
        onPress={() => {
          setTime(11600);
        }}
      >
        <SkipForward />
      </IconButton>
    </div>
  );
}

/** One part and no caption: the plainest slider. */
function Plain() {
  const [percent, setPercent] = useState(40);
  return (
    <div className="blocks-gallery-plain-slider">
      <Slider
        label="Brightness"
        segments={[{ from: 0, to: 100 }]}
        value={percent}
        step={5}
        valueText={(value) => `${String(Math.round(value))} percent`}
        onChange={setPercent}
      />
      <span className="gallery-note">{Math.round(percent)}%</span>
    </div>
  );
}

const page: GalleryPage = {
  area: 'building-blocks',
  part: 'Slider',
  states: [
    { name: 'Six parts, part way through the fourth', show: () => <Playback startAt={7400} /> },
    { name: 'At the start', show: () => <Playback startAt={400} /> },
    { name: 'At the end', show: () => <Playback startAt={11600} /> },
    { name: 'Disabled', show: () => <Playback startAt={3000} disabled /> },
    { name: 'One part, no caption', show: () => <Plain /> },
  ],
};

export default page;
