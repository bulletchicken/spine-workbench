import type { Reading } from '@/shapes';
export function degreeBand(reading: Extract<Reading, { kind: 'measured' }>, magnitude = false) {
  const raw = reading.valueDeg;
  const rounded = Math.sign(raw) * Math.floor(Math.abs(raw) + 0.5);
  const value = magnitude ? Math.abs(rounded) : rounded;
  return {
    value: `${value < 0 ? '−' : ''}${String(Math.abs(value))}°`,
    band: `± ${String(Math.ceil(Math.max(reading.band.p90 - raw, raw - reading.band.p10)))}°`,
  };
}
