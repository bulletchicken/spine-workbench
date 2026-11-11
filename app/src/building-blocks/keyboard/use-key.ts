// How a part answers keys: it registers them under a rank for as long as
// it is mounted. The rules of the walk are in keyboard.ts.

import { useEffect, useLayoutEffect, useRef } from 'react';
import type { KeyRank } from './key-ranks';
import { takeKeys, type KeyRun } from './keyboard';

/**
 * Answer `keys` at `rank` while this part is mounted. `run` returning
 * `false` means "not mine" and the walk goes on; `when`, if given, is
 * asked at each press. Both may be new functions on every render: the
 * registration, and its place among the takers of its rank, stay put.
 */
export function useKey(rank: KeyRank, keys: readonly string[], run: KeyRun, when?: () => boolean): void {
  const latest = useRef({ run, when });
  useLayoutEffect(() => {
    latest.current = { run, when };
  });

  // No key's name holds a line break, so the list survives the join.
  const spelled = keys.join('\n');
  useEffect(
    () =>
      takeKeys(
        rank,
        spelled.split('\n'),
        (event) => latest.current.run(event),
        () => latest.current.when?.() ?? true,
      ),
    [rank, spelled],
  );
}
