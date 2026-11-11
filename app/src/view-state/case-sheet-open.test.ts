import { describe, expect, it } from 'vitest';
import { createCaseSheetOpen } from './case-sheet-open';

describe('case sheet open flag', () => {
  it('starts shut and opens and shuts', () => {
    const sheet = createCaseSheetOpen();
    expect(sheet.getState().open).toBe(false);
    sheet.getState().setOpen(true);
    expect(sheet.getState().open).toBe(true);
    sheet.getState().setOpen(false);
    expect(sheet.getState().open).toBe(false);
  });
});
