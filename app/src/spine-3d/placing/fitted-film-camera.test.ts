import { describe, expect, it } from 'vitest';
import { fittedFilmCamera } from './fitted-film-camera';

describe('a film stood from the fit', () => {
  it('uses the effective stored spacings and upright origin, swapping spacings on odd turns only', () => {
    for (const quarterTurns of [0, 1, 2, 3] as const) {
      const camera = fittedFilmCamera({ imageRight: 'anterior', quarterTurns }, [300, 700], { mmPerPx: [0.42, 0.63], originPx: [27, 340], facingAssumed: false });
      expect(camera?.uprightMmPerPx).toEqual(quarterTurns % 2 === 0 ? [0.42, 0.63] : [0.63, 0.42]);
      expect(camera?.origin).toEqual([27, 340]);
    }
  });
});
