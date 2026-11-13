// The three parts of the room the frame mounts but does not own, found by
// their paths, as steps and jobs are. So the frame is whole before any of
// them exists, and no slice edits a shared file to mount its part.
//
//   bench/Bench.tsx               in the bench region. Until it exists: an empty bench.
//   case-sheet/CaseSheet.tsx      over the bench only; it shows itself when the
//                                 sheet is open. Until it exists: Save does nothing.
//   reopen-case/OpeningCase.tsx   round the step's Screen in the guide column; it
//                                 shows its own guide while a case is being opened
//                                 or was refused, otherwise its children. Until it
//                                 exists: the step's Screen alone.
//
// Each file default-exports its component. The bench and the sheet take
// no props; the opening screen takes the step's Screen as its children.

import type { ComponentType, ReactNode } from 'react';

export interface RoomParts {
  Bench: ComponentType | null;
  CaseSheet: ComponentType | null;
  OpeningCase: ComponentType<{ children: ReactNode }> | null;
}

/** Where each part is found, from this file. */
export const ROOM_PART_PATHS = {
  Bench: '../bench/Bench.tsx',
  CaseSheet: '../case-sheet/CaseSheet.tsx',
  OpeningCase: '../reopen-case/OpeningCase.tsx',
} as const satisfies Record<keyof RoomParts, string>;

/** The parts among the files found, each `null` until its file exists. */
export function roomPartsFrom(found: Readonly<Record<string, unknown>>): RoomParts {
  return {
    Bench: (found[ROOM_PART_PATHS.Bench] as RoomParts['Bench'] | undefined) ?? null,
    CaseSheet: (found[ROOM_PART_PATHS.CaseSheet] as RoomParts['CaseSheet'] | undefined) ?? null,
    OpeningCase: (found[ROOM_PART_PATHS.OpeningCase] as RoomParts['OpeningCase'] | undefined) ?? null,
  };
}

/** The room's parts as the app has them. */
export function findRoomParts(): RoomParts {
  return roomPartsFrom(
    import.meta.glob(['../bench/Bench.tsx', '../case-sheet/CaseSheet.tsx', '../reopen-case/OpeningCase.tsx'], { eager: true, import: 'default' }),
  );
}
