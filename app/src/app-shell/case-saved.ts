// What the top bar says about the case under its name, from the case's
// version and the mark its last save or open left.

import type { SavedMark } from '@/view-state/saved-state';
import { saveStatus } from '@/view-state/saved-state';

/**
 * - `not-saved-yet`: nothing has been saved, and there is nothing to save.
 * - `changes-not-saved`: the case holds work that is on no disk, whether
 *   it was never saved or was changed since.
 * - `saved-just-now`: saved, and not changed since.
 * - `as-saved`: opened from a file, and not changed since.
 */
export type CaseSaved = 'not-saved-yet' | 'changes-not-saved' | 'saved-just-now' | 'as-saved';

/** The state of a case at `version`; `holdsWork` is whether it has any film in it yet. */
export function caseSaved(mark: SavedMark | null, version: number, holdsWork: boolean): CaseSaved {
  switch (saveStatus(mark, version)) {
    case 'not-saved-yet':
      return holdsWork ? 'changes-not-saved' : 'not-saved-yet';
    case 'changes-not-saved':
      return 'changes-not-saved';
    case 'up-to-date':
      return mark?.how === 'opened' ? 'as-saved' : 'saved-just-now';
  }
}

/** The case is on disk exactly as it is on screen: the dot beside its name is lit. */
export function isOnDisk(saved: CaseSaved): boolean {
  return saved === 'saved-just-now' || saved === 'as-saved';
}
