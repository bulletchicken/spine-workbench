// The one way a FilmLayer or a FilmHud gets its data.
//
// The frozen FilmLayer is `{ id, Draw }`, and Draw receives only `space`.
// A Draw written afresh on every render, closing over its data, is a new
// component type each time, so React would unmount and mount the whole
// layer on every change: a focused pin would lose focus, an animation
// would restart. So the layer made here keeps one identity, and one Draw,
// for the life of its caller. Its data goes into a small store after each
// render (only when a field actually changed), and Draw reads it from
// there, so new data re-renders the part in place. Draw is memoised, so
// a viewer that renders for some other reason (a tool's hover, a
// right-drag) re-renders a layer only when its space is new.
//
//   const marks = useLayer('marks', MarksOverlay, { vertebrae, hot });
//   ... layers: [marks]

import { createElement, memo, useLayoutEffect, useMemo, useState, useSyncExternalStore, type ComponentType } from 'react';
import type { FilmHud, FilmHudProps, FilmLayer, FilmLayerProps } from './film-space';

/** What a slot holds: the part to draw and its props. */
interface Held<Part, Props> {
  Part: Part;
  props: Props;
}

interface Slot<Part, Props> {
  read: () => Held<Part, Props>;
  write: (Part: Part, props: Props) => void;
  subscribe: (listener: () => void) => () => void;
}

function makeSlot<Part, Props extends object>(Part: Part, props: Props): Slot<Part, Props> {
  let held: Held<Part, Props> = { Part, props };
  const listeners = new Set<() => void>();
  return {
    read: () => held,
    write(nextPart, nextProps) {
      if (nextPart === held.Part && sameFields(nextProps, held.props)) return;
      held = { Part: nextPart, props: nextProps };
      for (const listener of listeners) listener();
    },
    subscribe(listener) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
  };
}

function sameFields(a: object, b: object): boolean {
  const aKeys = Object.keys(a);
  if (aKeys.length !== Object.keys(b).length) return false;
  return aKeys.every((key) => Object.hasOwn(b, key) && Object.is((a as Record<string, unknown>)[key], (b as Record<string, unknown>)[key]));
}

function drawFrom<Props extends object, Given extends object>(slot: Slot<ComponentType<Props & Given>, Props>, name: string): ComponentType<Given> {
  function Draw(given: Given) {
    const { Part, props } = useSyncExternalStore(slot.subscribe, slot.read, slot.read);
    return createElement(Part, { ...props, ...given });
  }
  const remembered = memo(Draw);
  remembered.displayName = name;
  return remembered;
}

/** A FilmLayer drawing `Part` with `props`, whose identity never changes. */
export function useLayer<Props extends object>(id: string, Part: ComponentType<Props & FilmLayerProps>, props: Props): FilmLayer {
  const [{ slot, Draw }] = useState(() => {
    const made = makeSlot(Part, props);
    return { slot: made, Draw: drawFrom<Props, FilmLayerProps>(made, `Layer(${id})`) };
  });
  useLayoutEffect(() => {
    slot.write(Part, props);
  });
  return useMemo(() => ({ id, Draw }), [id, Draw]);
}

/** A FilmHud drawing `Part` with `props`, whose identity never changes. */
export function useHud<Props extends object>(id: string, Part: ComponentType<Props & FilmHudProps>, props: Props): FilmHud {
  const [{ slot, Draw }] = useState(() => {
    const made = makeSlot(Part, props);
    return { slot: made, Draw: drawFrom<Props, FilmHudProps>(made, `Hud(${id})`) };
  });
  useLayoutEffect(() => {
    slot.write(Part, props);
  });
  return useMemo(() => ({ id, Draw }), [id, Draw]);
}
