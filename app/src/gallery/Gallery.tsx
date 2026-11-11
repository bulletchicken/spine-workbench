import { useEffect, useLayoutEffect, useRef, useState, type MouseEvent } from 'react';
import { Button } from '@/building-blocks/Button';
import type { MotionMode } from '@/look-and-feel/data-attributes';
import { findPages, type FoundPage } from './find-pages';
import './Gallery.css';
import { GALLERY_WIDTHS, searchFor, viewFrom, type GalleryView } from './gallery-view';
import { GalleryStage } from './GalleryStage';

const MOTIONS: readonly { mode: MotionMode; name: string }[] = [
  { mode: 'full', name: 'Full' },
  { mode: 'reduced', name: 'Reduced' },
];

/** The pages of each area, in the order found. */
function byArea(pages: readonly FoundPage[]): [string, FoundPage[]][] {
  const areas = new Map<string, FoundPage[]>();
  for (const found of pages) areas.set(found.page.area, [...(areas.get(found.page.area) ?? []), found]);
  return [...areas];
}

/** True while the system asks for reduced motion. */
function useSystemReducedMotion(): boolean {
  const [reduced, setReduced] = useState(() => window.matchMedia('(prefers-reduced-motion: reduce)').matches);
  useEffect(() => {
    const asked = window.matchMedia('(prefers-reduced-motion: reduce)');
    const onChange = () => {
      setReduced(asked.matches);
    };
    asked.addEventListener('change', onChange);
    return () => {
      asked.removeEventListener('change', onChange);
    };
  }, []);
  return reduced;
}

/**
 * The frame a page is shown in: a real window of the chosen width, so the
 * one breakpoint and every media query answer as they would in the app.
 * When the gallery has less room than that, the frame is drawn smaller,
 * never reflowed.
 */
function PageFrame({ view, title }: { view: GalleryView; title: string }) {
  const room = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ width: 0, height: 0 });
  useLayoutEffect(() => {
    const element = room.current;
    if (element === null) return;
    const measure = () => {
      const inside = getComputedStyle(element);
      setSize({
        width: element.clientWidth - parseFloat(inside.paddingLeft) - parseFloat(inside.paddingRight),
        height: element.clientHeight - parseFloat(inside.paddingTop) - parseFloat(inside.paddingBottom),
      });
    };
    const watch = new ResizeObserver(measure);
    watch.observe(element);
    return () => {
      watch.disconnect();
    };
  }, []);

  const scale = size.width === 0 ? 1 : Math.min(1, size.width / view.width);
  return (
    <div className="gallery-frame-room">
      <p className="gallery-frame-note">
        {view.width} px wide{scale < 1 ? `, drawn at ${String(Math.round(scale * 100))}%` : ''}
      </p>
      <div className="gallery-frame-fit" ref={room}>
        <iframe
          className="gallery-frame"
          title={title}
          src={searchFor({ ...view, alone: true })}
          style={{ width: view.width, height: size.height / scale, transform: `scale(${String(scale)})` }}
        />
      </div>
    </div>
  );
}

/**
 * Every part in every state, at ?gallery in development. The index on
 * the left lists each `*.gallery.tsx` page by area; the switches choose
 * the width the page is framed at and whether motion is reduced. With
 * `&alone` in the address it is the page and nothing else, at the
 * window's own size.
 */
export function Gallery() {
  const [pages] = useState(findPages);
  const [view, setView] = useState(() => viewFrom(window.location.search));
  const systemReduced = useSystemReducedMotion();
  useEffect(() => {
    window.history.replaceState(null, '', searchFor(view));
  }, [view]);

  const found = pages.find(({ id }) => id === view.page) ?? pages[0];
  if (found === undefined) {
    return (
      <main className="gallery-empty">
        <p>No gallery pages yet. A file named *.gallery.tsx, anywhere under src/, shows up here.</p>
      </main>
    );
  }
  if (view.alone) return <GalleryStage found={found} motion={systemReduced ? 'reduced' : view.motion} />;

  const goTo = (id: string) => (event: MouseEvent) => {
    // A plain press stays in this window; with a modifier the link opens as links do.
    if (event.metaKey || event.ctrlKey || event.shiftKey || event.button !== 0) return;
    event.preventDefault();
    setView({ ...view, page: id });
  };

  return (
    <div className="gallery">
      <header className="gallery-bar">
        <div className="gallery-name">
          <b>Gallery</b>
          <span className="gallery-tag">dev only</span>
        </div>
        <div className="gallery-switches">
          <div className="gallery-switch" role="group" aria-label="Width">
            <span className="gallery-switch-name">Width</span>
            {GALLERY_WIDTHS.map((width) => (
              <Button
                key={width}
                look="text"
                aria-pressed={view.width === width}
                onPress={() => {
                  setView({ ...view, width });
                }}
              >
                {width}
              </Button>
            ))}
          </div>
          <div className="gallery-switch" role="group" aria-label="Motion">
            <span className="gallery-switch-name">Motion</span>
            {MOTIONS.map(({ mode, name }) => (
              <Button
                key={mode}
                look="text"
                aria-pressed={view.motion === mode}
                onPress={() => {
                  setView({ ...view, motion: mode });
                }}
              >
                {name}
              </Button>
            ))}
          </div>
          <a className="gallery-alone" href={searchFor({ ...view, page: found.id, alone: true })} target="_blank" rel="noreferrer">
            Open alone
          </a>
        </div>
      </header>

      <nav className="gallery-index" aria-label="Parts">
        {byArea(pages).map(([area, parts]) => (
          <section key={area}>
            <h2 className="gallery-area">{area}</h2>
            <ul>
              {parts.map(({ id, page }) => (
                <li key={id}>
                  <a href={searchFor({ ...view, page: id })} aria-current={id === found.id ? 'page' : undefined} onClick={goTo(id)}>
                    {page.part}
                    <span className="gallery-count" aria-hidden="true">
                      {page.states.length}
                    </span>
                  </a>
                </li>
              ))}
            </ul>
          </section>
        ))}
      </nav>

      <PageFrame view={{ ...view, page: found.id }} title={`${found.page.area}: ${found.page.part}`} />
    </div>
  );
}
