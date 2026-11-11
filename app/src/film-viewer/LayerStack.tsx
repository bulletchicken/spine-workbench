// The layers over a film: one SVG the size of the viewport, and in it one
// group per FilmLayer, each set to the film's matrix, so everything a
// layer draws is in film pixels and moves with the film. The tool's
// preview is the top group. Strokes keep their width on screen at any
// zoom (LayerStack.css), and nothing here takes a press: the viewer
// hands presses to the tool.

import type { ReactNode } from 'react';
import type { FilmLayer, FilmSpace, Matrix } from './film-space';
import './LayerStack.css';

/** An SVG transform attribute for a matrix. */
export function matrixAttribute(matrix: Matrix): string {
  return `matrix(${matrix.join(' ')})`;
}

interface LayerStackProps {
  space: FilmSpace;
  layers: readonly FilmLayer[];
  /** What the tool in hand would do, drawn above every layer. */
  preview?: ReactNode;
}

export function LayerStack({ space, layers, preview }: LayerStackProps) {
  const transform = matrixAttribute(space.matrix);
  return (
    <svg className="layer-stack" aria-hidden={layers.length === 0 && preview == null ? true : undefined}>
      {layers.map(({ id, Draw }) => (
        <g key={id} className="layer-stack__layer" data-layer={id} transform={transform}>
          <Draw space={space} />
        </g>
      ))}
      {preview != null && (
        <g className="layer-stack__layer" data-layer="tool-preview" transform={transform}>
          {preview}
        </g>
      )}
    </svg>
  );
}
