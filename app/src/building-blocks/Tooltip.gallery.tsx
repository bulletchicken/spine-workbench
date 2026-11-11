import { useEffect, useRef, type ReactNode } from 'react';
import type { GalleryPage } from '@/gallery/gallery-page';
import { Button } from './Button';
import { Kbd } from './Kbd';
import { Tooltip } from './Tooltip';
import './Tooltip.gallery.css';

const SIDES = ['below', 'above', 'left', 'right'] as const;
const ALIGNS = ['start', 'centre', 'end'] as const;

/**
 * Shows the tooltips inside it from the start, as if the pointer were
 * resting on each control: the page tells each one the pointer has come.
 * They are put away as usual, by the pointer leaving, a press or Escape.
 */
function PointerResting({ children }: { children: ReactNode }) {
  const around = useRef<HTMLDivElement>(null);
  useEffect(() => {
    for (const anchor of around.current?.querySelectorAll('.tooltip-anchor') ?? []) {
      anchor.dispatchEvent(new PointerEvent('pointerover', { bubbles: true }));
    }
  }, []);
  return (
    <div ref={around} className="blocks-gallery-resting">
      {children}
    </div>
  );
}

const page: GalleryPage = {
  area: 'building-blocks',
  part: 'Tooltip',
  states: [
    {
      name: 'Shown: a name, on each side',
      show: () => (
        <PointerResting>
          <div className="blocks-gallery-spread">
            {SIDES.map((side) => (
              <Tooltip key={side} tip={`Opens ${side}`} side={side}>
                <Button>{side}</Button>
              </Tooltip>
            ))}
          </div>
        </PointerResting>
      ),
    },
    {
      name: 'Shown: lined up with the start, the centre and the end',
      show: () => (
        <PointerResting>
          <div className="blocks-gallery-spread">
            {ALIGNS.map((align) => (
              <Tooltip key={align} tip="Undo: Split T12" align={align}>
                <Button>{align}</Button>
              </Tooltip>
            ))}
          </div>
        </PointerResting>
      ),
    },
    {
      name: 'Shown: help, a sentence or two beside a tool',
      show: () => (
        <PointerResting>
          <div className="blocks-gallery-help">
            <Tooltip
              look="help"
              side="right"
              tip={
                <>
                  <b>Boundary wand</b> Click a disc gap to split one vertebra into two. Click a boundary to merge two into one.
                </>
              }
            >
              <Button>Boundary</Button>
            </Tooltip>
          </div>
        </PointerResting>
      ),
    },
    {
      name: 'Shown: a name with its key',
      show: () => (
        <PointerResting>
          <div className="blocks-gallery-help">
            <Tooltip
              tip={
                <span className="blocks-gallery-tip-key">
                  Undo: Split T12 <Kbd keys={['Mod', 'z']} look="bare" />
                </span>
              }
              align="start"
            >
              <Button>Undo</Button>
            </Tooltip>
          </div>
        </PointerResting>
      ),
    },
    {
      name: 'Put away until the pointer or Tab comes',
      show: () => (
        <>
          <Tooltip tip="Undo: Split T12" align="start">
            <Button>Undo</Button>
          </Tooltip>
          <Tooltip tip="Nothing to redo" align="start">
            <Button>Redo</Button>
          </Tooltip>
        </>
      ),
    },
  ],
};

export default page;
