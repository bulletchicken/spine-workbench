// A path as a 2D canvas builds one, flattened into straight pieces as it
// is built: lines, curves, ellipses, arcs and rounded boxes become lists of
// points. Each list is one subpath; a closed one joins its last point to
// its first.

export interface Subpath {
  /** x, y, x, y, … */
  points: number[];
  closed: boolean;
}

/** Straight pieces per curve: fine enough that no film shows a corner. */
const CURVE_PIECES = 24;

export class Path {
  readonly subpaths: Subpath[] = [];

  private current(): Subpath | undefined {
    return this.subpaths[this.subpaths.length - 1];
  }

  private last(): [number, number] | undefined {
    const points = this.current()?.points;
    if (points === undefined || points.length < 2) return undefined;
    return [points[points.length - 2] ?? 0, points[points.length - 1] ?? 0];
  }

  moveTo(x: number, y: number): void {
    this.subpaths.push({ points: [x, y], closed: false });
  }

  lineTo(x: number, y: number): void {
    const current = this.current();
    if (current === undefined || current.closed) this.moveTo(x, y);
    else current.points.push(x, y);
  }

  quadraticCurveTo(cx: number, cy: number, x: number, y: number): void {
    const [x0, y0] = this.last() ?? [cx, cy];
    for (let k = 1; k <= CURVE_PIECES; k++) {
      const t = k / CURVE_PIECES;
      const u = 1 - t;
      this.lineTo(u * u * x0 + 2 * u * t * cx + t * t * x, u * u * y0 + 2 * u * t * cy + t * t * y);
    }
  }

  bezierCurveTo(c1x: number, c1y: number, c2x: number, c2y: number, x: number, y: number): void {
    const [x0, y0] = this.last() ?? [c1x, c1y];
    for (let k = 1; k <= CURVE_PIECES; k++) {
      const t = k / CURVE_PIECES;
      const u = 1 - t;
      const a = u * u * u;
      const b = 3 * u * u * t;
      const c = 3 * u * t * t;
      const d = t * t * t;
      this.lineTo(a * x0 + b * c1x + c * c2x + d * x, a * y0 + b * c1y + c * c2y + d * y);
    }
  }

  /**
   * Part of an ellipse, clockwise on the image from `start` to `end`. Like
   * a canvas, it joins the current point with a line when there is one; a
   * sweep of a whole turn or more is the whole ellipse.
   */
  ellipse(x: number, y: number, rx: number, ry: number, rotation: number, start: number, end: number): void {
    const whole = end - start >= 2 * Math.PI;
    const sweep = whole ? 2 * Math.PI : end - start;
    const pieces = Math.max(16, Math.ceil((Math.abs(sweep) * Math.max(rx, ry)) / 2));
    const cos = Math.cos(rotation);
    const sin = Math.sin(rotation);
    const current = this.current();
    const joins = current !== undefined && !current.closed;
    for (let k = 0; k <= pieces; k++) {
      if (whole && k === pieces) break;
      const t = start + (sweep * k) / pieces;
      const ex = rx * Math.cos(t);
      const ey = ry * Math.sin(t);
      const px = x + ex * cos - ey * sin;
      const py = y + ex * sin + ey * cos;
      if (k === 0 && !joins) this.moveTo(px, py);
      else this.lineTo(px, py);
    }
    if (whole) this.closePath();
  }

  arc(x: number, y: number, r: number, start: number, end: number): void {
    this.ellipse(x, y, r, r, 0, start, end);
  }

  roundRect(x: number, y: number, w: number, h: number, r: number): void {
    const corner = (cx: number, cy: number, from: number) => {
      for (let k = 0; k <= 6; k++) {
        const t = from + (Math.PI / 2) * (k / 6);
        this.lineTo(cx + r * Math.cos(t), cy + r * Math.sin(t));
      }
    };
    this.moveTo(x + r, y);
    corner(x + w - r, y + r, -Math.PI / 2);
    corner(x + w - r, y + h - r, 0);
    corner(x + r, y + h - r, Math.PI / 2);
    corner(x + r, y + r, Math.PI);
    this.closePath();
  }

  closePath(): void {
    const current = this.current();
    if (current !== undefined) current.closed = true;
  }
}
