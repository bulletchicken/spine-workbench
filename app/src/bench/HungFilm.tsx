import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import type { FilmKind } from '@/shapes';
import { joinLoop } from '@/choreography/one-loop';
import { numberOf } from '@/choreography/timing';
import { fit, lookPoint, zoomOverFit, type CameraFrame, type Viewport } from '@/film-viewer/camera/camera';
import { cameras } from '@/film-viewer/camera/cameras';
import { screenMatrix } from '@/film-viewer/film-space';
import type { FilmLayer, FilmPlacement, FilmSize } from '@/film-viewer/film-space';
import { FilmViewer, type FilmViewerHandle } from '@/film-viewer/FilmViewer';
import type { DrawState } from '@/film-viewer/gpu/film-painter';
import { STAGE_WORDS } from '@/words/stage';
import { spineFraming, vertebraFraming, type FilmFrame } from './film-framing';
import { motionIsReduced } from './glide-cameras';

interface HungFilmProps {
  view?: import('@/film-viewer/film-space').FilmView | undefined;
  kind: FilmKind;
  size: FilmSize;
  pixels: Uint16Array;
  thumbnail: Uint8Array;
  layers: readonly FilmLayer[];
  frame: FilmFrame;
  glide: boolean;
}

const FRAMES_TO_FIRST_DRAW = 2;
const AS_STORED = { quarterTurns: 0, mirrored: false } as const;
const NO_MARGINS = { top: 0, right: 0, bottom: 0, left: 0 };

function sameFrame(a: FilmFrame, b: FilmFrame): boolean {
  if (a.kind === 'whole' || b.kind === 'whole') return a.kind === b.kind;
  if (a.kind === 'spine' && b.kind === 'spine') return a.names === b.names && a.most === b.most && a.box.left === b.box.left && a.box.right === b.box.right && a.box.top === b.box.top && a.box.bottom === b.box.bottom;
  if (a.kind === 'vertebra' && b.kind === 'vertebra') return a.names === b.names && a.over === b.over && a.centre[0] === b.centre[0] && a.centre[1] === b.centre[1];
  return false;
}

function pngAddress(png: Uint8Array): string {
  let text = '';
  for (let from = 0; from < png.length; from += 8192) text += String.fromCharCode(...png.subarray(from, from + 8192));
  return `data:image/png;base64,${btoa(text)}`;
}

/** The thumbnail keeps the viewer's film box, rather than the tile's aspect. */
function placePicture(picture: HTMLImageElement | null, placement: FilmPlacement): void {
  if (picture === null) return;
  if (placement.view.quarterTurns === 0 && !placement.view.mirrored) {
    picture.style.left = `${String(placement.pan[0])}px`;
    picture.style.top = `${String(placement.pan[1])}px`;
    picture.style.width = `${String(placement.size.width * placement.zoom)}px`;
    picture.style.height = `${String(placement.size.height * placement.zoom)}px`;
    picture.style.transform = '';
    return;
  }
  const [a, b, c, d, e, f] = screenMatrix(placement);
  picture.style.left = '0';
  picture.style.top = '0';
  picture.style.width = `${String(placement.size.width)}px`;
  picture.style.height = `${String(placement.size.height)}px`;
  picture.style.transformOrigin = '0 0';
  picture.style.transform = `matrix(${[a, b, c, d, e - (a + c) / 2, f - (b + d) / 2].join(',')})`;
}

export function HungFilm({ kind, size, pixels, thumbnail, layers, frame, glide, view = AS_STORED }: HungFilmProps) {
  const root = useRef<HTMLDivElement>(null);
  const viewer = useRef<FilmViewerHandle>(null);
  const picture = useRef<HTMLImageElement>(null);
  const viewport = useRef<Viewport | null>(null);
  const measures = useRef<{ nameRoom: number; filmHeight: number } | null>(null);
  const [start] = useState(() => fit({ size, view, viewport: { width: 0, height: 0 }, margins: NO_MARGINS }));
  const [drawState, setDrawState] = useState<DrawState>('drawn');
  const [drawnPixels, setDrawnPixels] = useState<Uint16Array | null>(null);
  const thumbnailHidden = drawnPixels === pixels && drawState === 'drawn';
  const pictureUrl = useMemo(() => pngAddress(thumbnail), [thumbnail]);

  useEffect(() => {
    let frames = 0;
    const leave = joinLoop(() => {
      frames += 1;
      if (frames < FRAMES_TO_FIRST_DRAW) return;
      leave();
      setDrawnPixels(pixels);
    });
    return leave;
  }, [pixels]);

  useLayoutEffect(() => cameras.subscribe((which, moving) => {
    if (which !== kind) return;
    const placement = cameras.placementOf(kind);
    if (placement === null) return;
    viewer.current?.place(placement, moving);
    placePicture(picture.current, placement);
  }), [kind]);

  useLayoutEffect(() => () => { cameras.drop(kind); }, [kind]);

  const frameCamera = useCallback((camera: CameraFrame, moving: boolean) => {
    const part = root.current;
    if (part === null) return;
    const travel = moving && !motionIsReduced(part);
    if (frame.kind === 'whole') {
      cameras.fit(kind, travel);
      return;
    }
    measures.current ??= { nameRoom: numberOf(part, '--name-room'), filmHeight: numberOf(part, '--film-h') };
    const nameRoom = frame.names ? measures.current.nameRoom * size.height / measures.current.filmHeight : 0;
    const target = frame.kind === 'spine' ? spineFraming(camera, frame.box, nameRoom, frame.most) : vertebraFraming(camera, frame.centre, nameRoom, frame.over);
    cameras.travelTo(kind, target.point, target.over, travel);
  }, [frame, kind, size.height]);

  const hang = useCallback((box: Viewport) => {
    const before = cameras.frameOf(kind);
    const placement = cameras.placementOf(kind);
    const moving = cameras.isGliding(kind);
    const from = before === null || placement === null ? null : { point: lookPoint(before, placement), over: zoomOverFit(before, placement) };
    const camera: CameraFrame = { size, view, viewport: box, margins: NO_MARGINS };
    cameras.hang(kind, camera);
    // A new tile box must not finish a push-in that is still under way.
    if (moving && from !== null) cameras.travelTo(kind, from.point, from.over, false);
    frameCamera(camera, moving);
  }, [kind, size, frameCamera, view]);

  const lastFrame = useRef(frame);
  useLayoutEffect(() => {
    const camera = cameras.frameOf(kind);
    if (!sameFrame(lastFrame.current, frame) && camera !== null) frameCamera(camera, glide);
    lastFrame.current = frame;
  }, [frame, frameCamera, glide, kind]);

  const lastPicture = useRef({ size, pixels, view });
  useLayoutEffect(() => {
    const before = lastPicture.current;
    if (viewport.current !== null && (before.size.width !== size.width || before.size.height !== size.height || before.pixels !== pixels || before.view.quarterTurns !== view.quarterTurns)) hang(viewport.current);
    lastPicture.current = { size, pixels, view };
  }, [size, pixels, hang, view]);

  return (
    <div ref={root} className="hung-film">
      <img ref={picture} src={pictureUrl} alt="" hidden={thumbnailHidden} draggable={false}
        style={{ position: 'absolute', display: thumbnailHidden ? 'none' : 'block', maxWidth: 'none', pointerEvents: 'none', userSelect: 'none', width: size.width * start.zoom, height: size.height * start.zoom }} />
      <FilmViewer ref={viewer} pixels={pixels} placement={start} layers={layers} label={STAGE_WORDS.film(kind)} onDrawState={setDrawState}
        onViewport={(box) => { viewport.current = box; hang(box); }} />
    </div>
  );
}
