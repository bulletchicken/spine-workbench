import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { caseFileName } from './file-names';

// The name is the calendar date where the person is. The tests pin a zone
// that is behind UTC, so a late evening there is already the next day in
// UTC, and a name built from UTC would be caught.
const savedZone = process.env.TZ;
beforeAll(() => {
  process.env.TZ = 'America/Toronto';
});
afterAll(() => {
  if (savedZone === undefined) delete process.env.TZ;
  else process.env.TZ = savedZone;
});

describe('case file name', () => {
  it('is spine-case, the date and the extension, from the calendar date', () => {
    expect(caseFileName(new Date(2026, 10, 9))).toBe('spine-case-2026-11-09.spinecase');
    expect(caseFileName(new Date(2027, 0, 1))).toBe('spine-case-2027-01-01.spinecase');
  });

  it('is the same at any time of the day, since only the date is used', () => {
    expect(caseFileName(new Date(2026, 10, 9, 0, 0, 1))).toBe('spine-case-2026-11-09.spinecase');
    expect(caseFileName(new Date(2026, 10, 9, 23, 59, 59))).toBe('spine-case-2026-11-09.spinecase');
  });

  it('takes the date where the person is: 23:30 in Toronto on the 9th is still the 9th, though UTC is already the 10th', () => {
    const late = new Date(2026, 10, 9, 23, 30);
    expect(late.getUTCDate()).toBe(10);
    expect(caseFileName(late)).toBe('spine-case-2026-11-09.spinecase');
  });
});
