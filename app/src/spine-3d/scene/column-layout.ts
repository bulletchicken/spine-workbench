import { centreOf, fittedScale, type Bounds } from '../looking/framing';
import { toCanvas } from '../looking/lens';
import type { CanvasSize, View } from '../looking/view';

export interface Column { left: number; right: number; top: number; bottom: number }
export function columnCells(size: CanvasSize, count: number, gapPx: number, footPx: number): Column[] {
  if (![gapPx, footPx].every((n) => Number.isFinite(n) && n >= 0)) throw new RangeError('Invalid column spacing');
  const width = Math.max(0, (size.width - Math.max(0, count - 1) * gapPx) / Math.max(1, count));
  return Array.from({ length: count }, (_, i) => ({ left: i * (width + gapPx), right: i * (width + gapPx) + width, top: 0, bottom: Math.max(0, size.height - footPx) }));
}

/** Each box gets its own camera placement; turning and zoom remain one view. */
export function columnView(box: Bounds, cell: Column, size: CanvasSize, shared: View, ordinary: Bounds): View {
  const cellSize = { width: Math.max(1, cell.right - cell.left), height: Math.max(1, cell.bottom - cell.top) };
  const base = Math.max(1e-8, fittedScale(ordinary, size));
  const zoom = shared.pxPerMm[0] / base;
  const fitted = Math.max(1e-8, fittedScale(box, cellSize));
  let view: View = { ...shared, aim: centreOf(box), aimAt: [(cell.left + cell.right) / 2, (cell.top + cell.bottom) / 2], pxPerMm: [fitted, fitted] };
  // Fit the turned box too, including elevation and perspective, before applying the common zoom.
  for (let pass = 0; pass < 3; pass++) {
    let across = 0, up = 0;
    for (const x of [box.low[0], box.high[0]]) for (const y of [box.low[1], box.high[1]]) for (const z of [box.low[2], box.high[2]]) {
      const point = toCanvas(view, size, [x, y, z]); across = Math.max(across, Math.abs(point.x - view.aimAt[0])); up = Math.max(up, Math.abs(point.y - view.aimAt[1]));
    }
    const ratio = Math.min(1, cellSize.width / Math.max(1, 2 * across), cellSize.height / Math.max(1, 2 * up));
    view = { ...view, pxPerMm: [view.pxPerMm[0] * ratio, view.pxPerMm[1] * ratio] };
  }
  return { ...view, pxPerMm: [view.pxPerMm[0] * zoom, view.pxPerMm[1] * zoom] };
}
