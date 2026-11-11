// Who is asked first when a key is pressed. Highest first: the first part
// that takes the key stops the walk, so a key means one thing at a time.
//
//   always    Cmd or Ctrl+Z, with Shift for redo; Cmd or Ctrl+S
//   sheet     an open dialog: Escape closes, Enter presses its lit button
//   popover   an open popover: Escape
//   gesture   Escape drops a drag; the ruler's crosshair keys
//   choice    the two keys of the choice on screen
//   lit       Enter presses the lit button
//   review    Escape leaves the review
//   pass      Space plays and pauses the pass
//   tools     V, R, B, M
//   cameras   0, + and −

export const KEY_RANKS = ['always', 'sheet', 'popover', 'gesture', 'choice', 'lit', 'review', 'pass', 'tools', 'cameras'] as const;

export type KeyRank = (typeof KEY_RANKS)[number];
