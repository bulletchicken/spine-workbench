import { useState, type CSSProperties } from 'react';
import type { GalleryPage } from '@/gallery/gallery-page';
import './app-shell.gallery.css';
import type { RailStep } from './rail-steps';
import { TopBar } from './TopBar';

function Bar({ now = 'films', openAtStart = false }: { now?: string; openAtStart?: boolean }) {
  const [open, setOpen] = useState(openAtStart);
  const [chosen, setChosen] = useState(now);
  const rail: RailStep[] = ['films', 'find'].map((slug, at) => ({
    slug, number: at + 1, title: at === 0 ? 'Films' : 'Find',
    state: slug === chosen ? 'now' : at === 0 ? 'done' : 'todo',
    open: at === 0 || now === 'find',
  }));
  return (
    <div className="frame shell-gallery-bar" style={{ position: 'relative', width: '100%', minWidth: 0, height: open ? 260 : 80, '--col': 'min(720px, 100%)' } as CSSProperties}>
      <TopBar rail={rail} onGoToStep={setChosen} requests={{ sent: 0, blocked: 0 }} promiseOpen={open} onPromiseOpenChange={setOpen} />
    </div>
  );
}

const page: GalleryPage = {
  area: 'app-shell', part: 'TopBar', states: [
    { name: 'Films, with Find and 3D waiting', show: () => <Bar /> },
    { name: 'Find, with a way back to Films', show: () => <Bar now="find" /> },
    { name: 'The note under its badge', show: () => <Bar openAtStart /> },
  ],
};
export default page;
