import { describe, expect, it } from 'vitest';
import { caseSaved, isOnDisk } from './case-saved';

describe('what the top bar says of the case', () => {
  it('is "not saved yet" only while there is nothing to save', () => {
    expect(caseSaved(null, 0, false)).toBe('not-saved-yet');
    expect(caseSaved(null, 4, true)).toBe('changes-not-saved');
  });

  it('is "saved just now" until the next edit raises the version', () => {
    expect(caseSaved({ version: 12, how: 'saved' }, 12, true)).toBe('saved-just-now');
    expect(caseSaved({ version: 12, how: 'saved' }, 13, true)).toBe('changes-not-saved');
  });

  it('does not call a case that was opened "saved just now"', () => {
    expect(caseSaved({ version: 1, how: 'opened' }, 1, true)).toBe('as-saved');
    expect(caseSaved({ version: 1, how: 'opened' }, 2, true)).toBe('changes-not-saved');
  });

  it('lights the dot only when the screen and the disk agree', () => {
    expect(isOnDisk('saved-just-now')).toBe(true);
    expect(isOnDisk('as-saved')).toBe(true);
    expect(isOnDisk('changes-not-saved')).toBe(false);
    expect(isOnDisk('not-saved-yet')).toBe(false);
  });
});
