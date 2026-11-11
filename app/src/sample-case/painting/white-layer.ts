// A transparent layer that is only ever painted white at some opacity, as
// every layer of the mock's painter is. White paint makes colour and alpha
// one number, so a layer is one channel: how much white each pixel holds.
// It keeps a canvas's drawing calls, so the painter reads like the mock's.

import { fillCoverage, strokeCoverage, type Coverage } from './coverage';
import { Path } from './path';

/** The canvas compositing modes the mock's painter uses on its layers. */
export type Composite = 'source-over' | 'lighter' | 'destination-out';

export class WhiteLayer {
  readonly white: Float32Array;
  /** The opacity of the white that `fill` and `stroke` paint with. */
  fillAlpha = 1;
  strokeAlpha = 1;
  lineWidth = 1;
  composite: Composite = 'source-over';
  private path = new Path();

  constructor(
    readonly width: number,
    readonly height: number,
  ) {
    this.white = new Float32Array(width * height);
  }

  beginPath(): void {
    this.path = new Path();
  }

  moveTo(x: number, y: number): void {
    this.path.moveTo(x, y);
  }

  lineTo(x: number, y: number): void {
    this.path.lineTo(x, y);
  }

  quadraticCurveTo(cx: number, cy: number, x: number, y: number): void {
    this.path.quadraticCurveTo(cx, cy, x, y);
  }

  bezierCurveTo(c1x: number, c1y: number, c2x: number, c2y: number, x: number, y: number): void {
    this.path.bezierCurveTo(c1x, c1y, c2x, c2y, x, y);
  }

  ellipse(x: number, y: number, rx: number, ry: number, rotation: number, start: number, end: number): void {
    this.path.ellipse(x, y, rx, ry, rotation, start, end);
  }

  roundRect(x: number, y: number, w: number, h: number, r: number): void {
    this.path.roundRect(x, y, w, h, r);
  }

  closePath(): void {
    this.path.closePath();
  }

  fill(): void {
    this.paint(fillCoverage(this.path, this.width, this.height), this.fillAlpha);
  }

  stroke(): void {
    this.paint(strokeCoverage(this.path, this.lineWidth, this.width, this.height), this.strokeAlpha);
  }

  private paint({ x0, y0, width, height, cover }: Coverage, alpha: number): void {
    const white = this.white;
    for (let row = 0; row < height; row++) {
      const from = (y0 + row) * this.width + x0;
      for (let col = 0; col < width; col++) {
        const a = (cover[row * width + col] ?? 0) * alpha;
        if (a === 0) continue;
        const k = from + col;
        const was = white[k] ?? 0;
        if (this.composite === 'source-over') white[k] = was + a * (1 - was);
        else if (this.composite === 'lighter') white[k] = Math.min(1, was + a);
        else white[k] = was * (1 - a);
      }
    }
  }
}

/** A whole ellipse as the path, as the mock's `ell` helper makes one. */
export function ellipsePath(layer: WhiteLayer, x: number, y: number, rx: number, ry: number, rotation = 0): void {
  layer.beginPath();
  layer.ellipse(x, y, rx, ry, rotation, 0, Math.PI * 2);
}
